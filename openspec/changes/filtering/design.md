# Design

## Context

See proposal.md for motivation and specs/vcf-filtering/spec.md for the rules. Relevant current state:

- `search_features` (`seqa_core/src/api/search.rs:79`) dispatches VCF searches to `tabix_search` (`seqa_core/src/tabix/tabix_search.rs:79`). That function loads the index and header, then for each coordinate fetches chunks and calls `read_lines` (`:30`). `read_lines` parses every line as a `Box<dyn Feature>` and keeps the ones that overlap.
- `VcfLine` (`seqa_core/src/tabix/vcf.rs:36`) already exposes the parsed columns as public fields: `quality: Option<f32>`, `filter: Vec<String>` (split on `;`), `info: Vec<(String, String)>` (flags have an empty value), and `sample_data: Vec<Vec<(String, String)>>`.
- `TabixHeader` (`seqa_core/src/tabix/tabix_header.rs:24`) keeps the `#` header lines, including the `#CHROM` line that names the sample columns.
- The gene DB (`seqa_core/src/sqlite/genes.rs:35`) has one row per symbol with `chromosome`, `begin`, `end`. The positions are one-based closed: hg38 RefSeq BRCA1 begins at 43044295, which is NCBI's one-based start. `sqlite` is an optional, default-on feature.
- The CLI, axum and lambda all build `SearchOptions` and call `search_features`. axum and lambda are binary crates, so their tests must live inside their own source files.

## Goals / Non-Goals

**Goals:**
- One filter implementation in `seqa_core`, used by all three front ends.
- Every check that needs no record data finishes before any VCF records are fetched.
- Searches without a filter behave exactly as before.

**Non-Goals:**
- NOT.
- Using filter coordinates or genes to narrow which tabix chunks are fetched. Filters only post-filter records within the requested coordinates.
- Checking types against `##INFO` / `##FORMAT` header declarations. Types are checked against each record's actual values.
- Filtering any format other than VCF.
- Fixing the INFO parser's handling of values that contain `=` (see Risks).

## Decisions

**1. A new `seqa_core::filter` module with three files.**
- `filter/mod.rs`: the public, serde-able types `Filter`, `Condition`, `Operator` and `FilterValue`, plus `Filter::from_json(&str)` and `Filter::from_value(serde_json::Value)`. Both return `SearchError::InvalidFilter`.
- `filter/genotype.rs`: GT parsing and the six genotype classes.
- `filter/resolved.rs`: `ResolvedFilter`, which does validation, resolution and per-record matching.

*Alternative considered:* putting it in `tabix/`. The filter is part of the public API and is not tied to tabix internals, so it gets its own module next to `api`.

**2. Serde shape: an untagged outer enum and an internally tagged leaf enum.**
```rust
#[serde(untagged)]
pub enum Filter {
    And { #[serde(rename = "AND")] and: Vec<Filter> },   // deny_unknown_fields
    Or  { #[serde(rename = "OR")]  or:  Vec<Filter> },   // deny_unknown_fields
    Condition(Condition),
}
#[serde(tag = "type", rename_all = "UPPERCASE", deny_unknown_fields)]
pub enum Condition {
    Info { key, operator, value: Option<FilterValue> },
    Sample { #[serde(rename = "sampleId")] sample_id, key, operator, value: Option<FilterValue> },
    Qual { operator, value: FilterValue },
    Filters { operator, value: FilterValue },
    Genes { values: Vec<String> },
    Coordinates { operator, values: Vec<String> },
}
#[serde(untagged)] pub enum FilterValue { Number(f32), Text(String) }
```
`Operator` is a plain enum with serde renames (`"eq"`, `"!contains"`, `"!overlaps"`, `"het"`, …). Because the type is matched strictly in uppercase and unknown fields are denied, `"coordinates"`, `NOT` and stray properties all fail to deserialize.

Serde only checks shape. Rules that span several fields are checked in `ResolvedFilter::new` (decision 4): operator allowed for the type, value present or absent, numeric vs text, GT-only genotype operators, non-empty arrays.

*Alternative considered:* a hand-written deserializer for better messages. Untagged enums give a generic "did not match any variant" message for badly shaped documents. That is accepted for now (see Risks), because the field-level rules carry specific messages.

**3. JSON text and values enter core through `Filter::from_json` / `Filter::from_value`.**
- The CLI reads the spec file and calls `Filter::from_json`, so it needs no `serde_json` dependency.
- The axum and lambda request models hold `filter` as a `serde_json::Value` and call `Filter::from_value`. A badly shaped filter therefore becomes a `SearchError::InvalidFilter` and a 400 with a filter-specific message, not a generic body-parse rejection.

**4. `ResolvedFilter` is a separate compiled form, built once per search.**
`ResolvedFilter::new(filter, header, genome, gene_source)` walks the tree and returns a mirror of it with everything precomputed:
- SAMPLE `sampleId` is replaced by its column index from the `#CHROM` line (column 10 onwards).
- COORDINATES strings are parsed with `Coordinate::new(..., OneBasedClosed)`.
- GENES symbols are resolved through `sqlite::connect_genes` and `genes::get_gene_symbols` into `Coordinate`s tagged `OneBasedClosed`. Before that, the genome is checked against `hg19|hg38|grch37|grch38` and the source against `refseq|gencode`, because `connect_genes` silently falls back to a default database (`seqa_core/src/sqlite/mod.rs:31-36`, `:56-63`). The DB is opened once per search, not once per gene. Without the `sqlite` feature, a GENES filter returns `InvalidFilter`.
- Text filter values are lower-cased once.

`tabix_search` builds it right after the header is available and before the coordinate loop, which satisfies the "validate before reading records" requirement.

*Alternative considered:* evaluating the raw `Filter` directly. That would repeat the gene lookup and sample lookup for every record and push validation into the per-record path.

**5. Applying the filter is a VCF-only branch in `tabix_search`.**
- `search_features` returns `InvalidFilter` up front when `filter` is set and `output_format` is not `VCF`.
- In `tabix_search`, when `ResolvedFilter` is present, a new `read_filtered_vcf_lines(data, coordinate, &resolved)` is used in place of `data_to_lines`. It parses each line with `VcfLine::from_bytes`, keeps it only if `coordinate()` overlaps the requested coordinate and `resolved.matches(&line)?` is true, and writes it back with `to_string()`.
- `read_lines` and `data_to_lines` are unchanged, so searches without a filter are identical.

**6. Matching semantics** (from the spec):
- `matches` returns `Result<bool, SearchError>`. AND and OR short-circuit.
- A value lookup returns `None` for a missing key, an empty value or `.`.
  - Value operators return `false` on `None`.
  - Presence operators look only at whether there is a value. An INFO flag (empty value, key present) counts as present.
- Multi-value: split on `,`, skip `.` elements, and match if any element matches. If every element is `.`, the field is treated as missing.
- Numeric: `str::parse::<f32>()`. A parse failure on a non-missing element returns `InvalidFilter` naming the key and value.
- Text: compare `to_lowercase()` forms. `eq` and `neq` compare whole values; `contains` and `!contains` test for a substring.
- FILTERS: split each `filter` entry again on `,`, compare whole entries case-insensitively, and treat a single `.` entry as unset.
- QUAL uses `VcfLine::quality`. `None` (`.`) never matches. A non-numeric QUAL also parses to `None` today, so it is treated as missing rather than raising an error. That is malformed VCF and outside the INFO/SAMPLE type rule.
- GT parse: split on whichever of `|` or `/` appears; one or two alleles; each allele is `.` or an unsigned integer; anything else returns `InvalidFilter`. Phased means the separator is `|`.

**7. Overlap for COORDINATES and GENES uses SV breakpoints.**
A helper `breakpoints(&VcfLine) -> Option<(Coordinate, Coordinate)>` returns `Some` when INFO has a parseable `END`, or a parseable `SVLEN` (parsed as `i32`, absolute value):
- begin breakpoint = `CHROM:POS`
- end breakpoint = `(CHR2 or CHROM):(END or POS + |SVLEN|)`

Both are single-base `OneBasedClosed` coordinates, so `Coordinate::overlaps` can be reused (it ignores any `chr` prefix). Records without these use `VcfLine::coordinate()`. `!overlaps` is the negation of "overlaps any region".

**8. SVLEN bug fix in `VcfLine::get_length` (`seqa_core/src/tabix/vcf.rs:91-120`).**
The `else if` chain becomes a sequence of `if let ... && let Ok(...)` steps, so each source falls through to the next when it is missing or unparseable: `SVLEN` (as `i32`, `unsigned_abs`) → `CNVLEN` → `END` → longest alt → 1. This changes `search` results for deletions that have a negative `SVLEN`, which is intended.

**9. One error variant: `SearchError::InvalidFilter(String)`.**
It covers both a bad filter and a VCF value of the wrong type. Front ends map it to a client error: 400 in axum and lambda, and a non-zero exit with the message in the CLI. Other `SearchError`s keep their current mapping.

**10. `SearchOptions` additions.**
- `filter: Option<Filter>`, default `None`, with `set_filter(Filter)`.
- `gene_source: String`, default `"refseq"`, with `set_gene_source(&str)`, which stores it lower-cased.
- `genome` already exists. GENES requires it to be `Some`.

**11. Front ends.**
- **CLI:** a `Filter` subcommand whose arguments mirror `Search`, plus `--spec <path>` and `-s/--source` (default `refseq`). It reuses `print_output`. The flags are duplicated rather than shared through a new abstraction, because there are only two commands.
- **axum:** a `FilterRequest` model (`path`, `coordinates`, `filter: serde_json::Value`, `genome`, `gene_source`, `with_header`, `only_header`) and a `POST /filter` handler.
  - It rejects any path that does not end in `.vcf.gz`.
  - It reuses `populate_from_cache` / `write_back_to_cache`.
  - It maps `InvalidFilter` to 400.
- **lambda:** `SearchRequest` gains `filter: Option<serde_json::Value>` and `gene_source: Option<String>`. `search_handler` maps `InvalidFilter` to 400.

**12. Test fixture.**
A hand-written `seqa_core/tests/data/filter.vcf` with two samples, compressed with `bgzip` and indexed with `tabix -p vcf`. All three files are committed. It holds one record per case in the spec: flags, multi-value AF with `.`, `.` values, SVs with `END`, negative `SVLEN` and `CHR2`, FILTER with `;` and `,`, one record per genotype case, and records inside hg38 BRCA1. The `DP=high` type-error record sits on its own contig, so only the type-error test's region hits it.

All tests use local `file://` paths, so none need cloud credentials. Core tests live in `seqa_core/tests/filter.rs` and the CLI tests in `seqa_cli/tests/filter.rs`. The CLI tests run the built binary through `CARGO_BIN_EXE_seqa`. axum and lambda tests go in their existing in-file test modules, because they are binary crates.

## Risks / Trade-offs

- [Badly shaped JSON gets a generic serde message from the untagged enum] → Field-level rule errors carry specific messages. If users find the generic message confusing, a hand-written deserializer can replace the derive later without changing the JSON contract.
- [AND/OR short-circuit can hide a type error in a branch that is never evaluated] → Accepted. This is the usual short-circuit behaviour and matches the "first error wins" model. It is documented in the README.
- [A type error on one record fails the whole request] → This is what you asked for. The error message names the key and value so the user can fix the filter or the data.
- [`neq` on multi-value fields matches when any value differs, which surprises some users] → Same as bcftools. Documented with an example.
- [An SV that spans a region with neither breakpoint inside it does not overlap for COORDINATES or GENES] → This is the rule you asked for. It differs from `search`'s span overlap, and is documented.
- [The INFO parser drops a value that contains `=` (`seqa_core/src/tabix/vcf.rs:270-275`, e.g. `ANN=a=b` becomes a flag)] → Not fixed here, and flagged. Filters on such keys would see a flag rather than the value.
- [`get_length` computes `(end - position) + 1` in `u32`, which underflows when `END < POS` in malformed VCF] → Not changed here, and flagged.
- [Breaking `SearchOptions` change] → Major version bump for `seqa_core`. All in-repo callers are updated in this change.

## Migration Plan

This adds a feature and needs no data migration. External `seqa_core` users who build `SearchOptions` with a struct literal must add `filter: None, gene_source: "refseq".into()`. Rolling back means reverting the release; nothing is persisted.

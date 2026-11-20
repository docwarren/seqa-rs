# Proposal

## Why

Today seqa can only narrow VCF results by genomic region. Users who want, say, heterozygous PASS variants with QUAL above a threshold in a gene panel have to fetch every record in the region and filter downstream. A filter applied inside `seqa_core` lets the CLI, the axum server and the lambda return only the records the user cares about, with one set of rules shared by all three. The rules are described in `filter-spec.txt` in this change.

## What Changes

- **New filter language (seqa_core).** A filter is a JSON document. A leaf filter has the shape `{ "type", "key"?, "sampleId"?, "operator"?, "value" | "values" }`. Leaves combine with `{"AND": [...]}` and `{"OR": [...]}`, each taking a non-empty array. NOT is out of scope. Leaf types:
  - `INFO`: compare an INFO field by `key`.
  - `SAMPLE`: compare a FORMAT field for one sample, chosen by `sampleId`. On the `GT` key, the genotype operators `na`, `ref`, `hom`, `het`, `father` and `mother` are available, and only there.
  - `QUAL`: compare the QUAL column like a numeric INFO field.
  - `FILTERS`: `contains` / `!contains` against the FILTER column. Entries may be separated by `;` or `,`.
  - `GENES`: a list of gene symbols. Each is looked up in the embedded gene database (by genome and gene source) and kept only if the record overlaps it. No operator.
  - `COORDINATES`: `overlaps` / `!overlaps` against a list of regions.
- **Operators and matching:**
  - Text: `eq`, `neq`, `contains`, `!contains`. All text comparisons are case-insensitive.
  - Numeric (`f32`): `eq`, `neq`, `lt`, `gt`, `lte`, `gte`.
  - Presence: `exists` / `!exists`, for flag fields and presence checks.
  - Number vs text comparison is decided by the JSON value type: `10` is numeric, `"10"` is text.
  - Multi-value fields match the way bcftools does: the filter matches if any value matches.
  - Overlap for structural variants that have `END`/`SVLEN` matches when either the begin breakpoint or the end breakpoint overlaps.
- **Errors.** An incomplete or invalid filter returns a `SearchError`. So does a VCF value that does not match the type the filter expects. Examples of invalid filters: unknown type or operator, genotype operator on a key other than GT, unknown sample, unknown gene, GENES without a genome, or a filter on a file that is not a VCF.
- **Fix: SV length from `SVLEN`.** Today `SVLEN` is parsed as `u32`, so the negative `SVLEN` used for deletions fails to parse. The length then falls back to 1 without trying `END` (`seqa_core/src/tabix/vcf.rs:91-120`). The fix parses it as `i32` and uses its absolute value, falling back to `END` when `SVLEN` is missing or unparseable. This also corrects how `search` decides whether a deletion overlaps a region.
- **`SearchOptions`** gains `filter: Option<Filter>` and `gene_source` (default `refseq`). `search_features` applies the filter to VCF records after the region search. When `filter` is `None`, behaviour is unchanged.
- **CLI (`seqa_cli`).** New `filter` subcommand. It takes a file, one or more required coordinates, and a path to a JSON filter file, plus the same genome, header and no-cache options as `search`, and a gene-source option.
- **Axum (`seqa_axum`).** New `POST /filter` endpoint. It takes the `/search` body plus `filter`, `genome` and `gene_source`.
- **Lambda (`seqa_lambda`).** The search payload accepts optional `filter` and `gene_source` fields.

## Capabilities

### New Capabilities
- `vcf-filtering`: the JSON filter language for VCF records (leaf types, operators, genotype classes, AND/OR combinators), how each operator matches a record, how gene and coordinate filters are resolved, the validation errors, and how filters are passed in through the CLI `filter` command, the axum `/filter` endpoint and the lambda payload.

### Modified Capabilities
<!-- None: openspec/specs/ has no existing capabilities. -->

## Impact

- **seqa_core**: new `filter` module. Changes to `api/search_options.rs` (new fields and setters), `api/search_error.rs` (filter error variant), `api/search.rs` / `tabix/tabix_search.rs` (apply the filter to VCF records) and `tabix/vcf.rs` (the SV breakpoint and INFO lookups the filter needs). GENES filters depend on the optional `sqlite` feature. Without it, a GENES filter returns a `SearchError`.
- **seqa_cli**: new `Filter` subcommand in `src/main.rs`, reusing the existing output printing.
- **seqa_axum**: new route and request model in `src/main.rs` / `src/search/models.rs`, reusing the existing index and header cache.
- **seqa_lambda**: `SearchRequest` in `src/http_handler.rs` gains `filter` and `gene_source`.
- **Public API (breaking)**: `SearchOptions` gets new public fields, so code that builds it with a struct literal must add them. The new `SearchError` variant is not breaking, because the enum is `#[non_exhaustive]`. That makes this a major bump for `seqa_core` (9.0.1 → 10.0.0) and minor bumps for `seqa` CLI (3.1.0 → 3.2.0), `seqa_axum` (6.0.0 → 6.1.0) and `seqa_lambda` (4.0.0 → 4.1.0). New public API: the `filter` module (`Filter` and `Filter::from_json`).
- **Docs**: `filter-spec.txt` will be brought in line with the agreed rules once the specs are settled. The READMEs for seqa_core, seqa_cli and the axum/lambda APIs get filter sections.
- **Dependencies**: none new. `serde_json` is already used by seqa_core and the lambda. The CLI will need `serde_json` to read the filter file.

# Tasks

## 1. Setup and fixture

- [ ] 1.1 Branch off `main` as `feat/vcf-filtering` and check that `git branch --show-current` prints it
- [ ] 1.2 Write `seqa_core/tests/data/filter.vcf`: two samples, `##INFO`/`##FORMAT` lines, and one record per case in design decision 12:
  - flag `DB`
  - `AF=.,0.2` and `AF=0.1,0.6`
  - `DP=.`
  - case-mixed `SVTYPE` / `ANN` text
  - FILTER `.`, `PASS`, `PASSED`, `q10;LowQual`, `q10,LowQual`, `q10;FAIL`
  - SVs: `END=50000`; `SVLEN=-500` with no END; `SVLEN=abc` with an END; `CHR2=chr5;END=7000`
  - one record per GT case in the genotype scenarios
  - records inside and outside hg38 BRCA1 (`chr17:43044295-43170327`)
  - a `DP=high` record on its own contig

  Then run `bgzip -k` and `tabix -p vcf`, and check that `tabix seqa_core/tests/data/filter.vcf.gz chr1` lists the expected chr1 records

## 2. SVLEN length fix (tests first)

- [ ] 2.1 In a new `seqa_core/tests/vcf_length.rs`, add a `search_features` test on the fixture. A region `chr1:1450-1550` must return the deletion at `chr1:1000` that has `SVLEN=-500` and no END. A `SVLEN=abc;END=…` record must take its length from END. Confirm it fails with `cargo test -p seqa_core --test vcf_length`
- [ ] 2.2 Rewrite `VcfLine::get_length` (`seqa_core/src/tabix/vcf.rs:91-120`) as a fall-through chain: SVLEN parsed as `i32` (`unsigned_abs`) → CNVLEN → END → longest alt → 1 (design decision 8). Confirm 2.1 passes, and that `cargo test -p seqa_core --lib vcf` still passes the existing Manta/Canvas tests

## 3. Filter types, parsing and errors (tests first)

- [ ] 3.1 Create `seqa_core/tests/filter_parse.rs` with tests for `Filter::from_json`:
  - accepts every leaf shape in the "Leaf filter types and properties" requirement, plus nested AND/OR
  - rejects with `SearchError::InvalidFilter`: lowercase `type`, `NOT`, unknown operator, missing `key`, an unused property (`operator` on GENES), and malformed JSON

  Confirm the tests fail to compile or fail
- [ ] 3.2 Add `SearchError::InvalidFilter(String)` in `seqa_core/src/api/search_error.rs`, with a docstring and the message `Invalid filter: {0}`
- [ ] 3.3 Create `seqa_core/src/filter/mod.rs` with `Filter`, `Condition`, `Operator` and `FilterValue` (serde shape from design decision 2), plus `Filter::from_json` and `Filter::from_value`, all with docstrings. Export `pub mod filter` from `seqa_core/src/lib.rs`. Run `cargo test -p seqa_core --test filter_parse` and confirm 3.1 passes
- [ ] 3.4 Add `filter: Option<Filter>` and `gene_source: String` (default `"refseq"`) to `SearchOptions` (`seqa_core/src/api/search_options.rs:57`), with the setters `set_filter` and `set_gene_source` (lower-cases its input), and initialise both in `SearchOptions::new`. Add a test in `seqa_core/tests/filter_parse.rs` checking that the defaults are `None` / `"refseq"` and that both setters work. Check that `cargo build --workspace` succeeds

## 4. Validation and resolution (tests first)

- [ ] 4.1 Create `seqa_core/tests/filter.rs`. Add `search_features` tests on the fixture that expect `InvalidFilter`, one for each validation scenario in the spec:
  - empty AND/OR
  - a numeric operator with a string value
  - `contains` with a number value
  - a genotype operator on a key other than GT
  - `exists` with a `value`
  - unknown `sampleId`
  - a bad or empty COORDINATES list
  - an empty GENES list
  - GENES with no genome
  - an unknown gene
  - an unknown genome (`hg37`) or gene source (`gencod`)
  - a filter on a BAM (`SearchOptions` with `output_format` BAM)

  Confirm they fail
- [ ] 4.2 Implement `seqa_core/src/filter/resolved.rs`, building `ResolvedFilter::new` as in design decision 4:
  - sample index from the `#CHROM` header line
  - COORDINATES parsed as one-based closed
  - GENES resolved once per search through `sqlite::connect_genes` / `genes::get_gene_symbols`, gated on `#[cfg(feature = "sqlite")]`, after validating genome and source
  - every operator/value rule enforced

  In `search_features` (`seqa_core/src/api/search.rs:79`), reject a filter on a non-VCF format. Confirm 4.1 passes

## 5. Matching (tests first)

- [ ] 5.1 Add tests to `seqa_core/tests/filter.rs` covering every matching scenario in the spec, each running `search_features` on the fixture and checking which records come back:
  - numeric and text `eq`/`neq`
  - `lt`/`gt`/`lte`/`gte`
  - case-insensitive `eq`/`contains`/`!contains`
  - missing and `.` values for every operator
  - `exists`/`!exists` on INFO flags and SAMPLE values
  - bcftools-style multi-value matching, including `neq` and a skipped `.`
  - the six genotype classes (every listed GT)
  - QUAL
  - FILTERS with both delimiters, `PASSED` not matching `PASS`, and an unset `.`
  - COORDINATES `overlaps`/`!overlaps`
  - GENES BRCA1/BRCA2 on hg38 RefSeq and on GENCODE
  - SV begin/end breakpoint overlap, CHR2, negative SVLEN, and a span with no breakpoint inside the region
  - a nested AND/OR
  - `DP=high` returning `InvalidFilter`

  Confirm they fail
- [ ] 5.2 Implement `seqa_core/src/filter/genotype.rs` (GT parsing and the classes `na`, `ref`, `hom`, `het`, `father`, `mother`, per decision 6) and `ResolvedFilter::matches` (decisions 6 and 7), including the `breakpoints` helper. Use `begin`/`end` naming throughout
- [ ] 5.3 Add `read_filtered_vcf_lines` to `seqa_core/src/tabix/tabix_search.rs`. Build the `ResolvedFilter` after the header loads and before the coordinate loop (`tabix_search.rs:104-109`), and use it in place of `data_to_lines` when a filter is set (decision 5). Confirm 5.1 passes. Then confirm `cargo test -p seqa_core --test tabix` still passes, so unfiltered searches are unchanged
- [ ] 5.4 Add a test in `seqa_core/tests/filter.rs` showing that a filtered search with two coordinates returns two per-coordinate lists and the unchanged header. Confirm it passes

## 6. CLI `filter` command (tests first)

- [ ] 6.1 Add clap parse tests to the existing test module in `seqa_cli/src/main.rs:252`: `filter` with coordinates, `--spec`, `-r`, `-s`, `-w`; and failure when no coordinate is given. Create `seqa_cli/tests/filter.rs`, which runs the binary via `env!("CARGO_BIN_EXE_seqa")` on the fixture. It should check:
  - a PASS-only spec prints only PASS records
  - a missing spec file exits non-zero with an error
  - invalid spec JSON exits non-zero with an error

  Confirm the tests fail
- [ ] 6.2 Add the `Filter` subcommand to `seqa_cli/src/main.rs:37`. It reads `--spec` through `Filter::from_json`, sets `filter`, `genome` and `gene_source` on `SearchOptions`, reuses the existing header/no-cache handling, and calls `print_output`. Add docstrings to the new arguments. Run `cargo test -p seqa` and confirm 6.1 passes

## 7. Axum `POST /filter` (tests first)

- [ ] 7.1 Add tests to the test module in `seqa_axum/src/main.rs` using the fixture's local path:
  - 200 with only matching lines
  - 400 for a missing `filter`
  - 400 for an unknown operator
  - 400 for a `.bam` path
  - 400 for an unknown gene

  Confirm they fail
- [ ] 7.2 Add `FilterRequest` to `seqa_axum/src/search/models.rs` and a `feature_filter` handler plus `.route("/filter", post(feature_filter))` to `seqa_axum/src/main.rs` (decision 11). Map `SearchError::InvalidFilter` to `ApiError::BadRequest`. Run `cargo test -p seqa_axum` and confirm 7.1 passes

## 8. Lambda payload (tests first)

- [ ] 8.1 Add tests to the test module in `seqa_lambda/src/http_handler.rs`:
  - a payload with a FILTERS PASS filter returns only PASS lines from the fixture
  - an invalid filter (missing `key`) returns 400
  - an unfiltered payload is unchanged

  Confirm they fail
- [ ] 8.2 Add `filter: Option<serde_json::Value>` and `gene_source: Option<String>` to `SearchRequest` (`seqa_lambda/src/http_handler.rs:24`). Apply them in `search_handler` and map `InvalidFilter` to 400. Run `cargo test -p seqa_lambda` and confirm 8.1 passes

## 9. Documentation

- [ ] 9.1 Add a "Filtering" section to `seqa_core/README.md` covering:
  - the JSON shape
  - operators per type
  - genotype classes
  - missing/`.` rules
  - multi-value (bcftools) semantics
  - SV breakpoint overlap
  - errors, including the short-circuit note

  Make sure every example in it parses with `Filter::from_json`
- [ ] 9.2 Document `seqa filter` in `seqa_cli/README.md`: options table and an example spec file. Check the documented command against the fixture
- [ ] 9.3 Document `POST /filter` and the lambda `filter` / `gene_source` fields in the root `README.md` "API Server" section. Make sure the example body succeeds in the 7.1 test

## 10. Versioning and full verification

- [ ] 10.1 Bump versions: `seqa_core` 9.0.1 → 10.0.0, `seqa` 3.1.0 → 3.2.0, `seqa_axum` 6.0.0 → 6.1.0, `seqa_lambda` 4.0.0 → 4.1.0. Check that `cargo build --workspace` succeeds and `Cargo.lock` is updated
- [ ] 10.2 Run `cargo clippy --workspace --all-targets` and fix every warning
- [ ] 10.3 Run `cargo test --workspace` and confirm everything passes
- [ ] 10.4 Commit as `feat: vcf filtering`

## Workflow follow-up

- Check with the user before pushing. The root workspace has no version of its own, so tag only if the user asks for one.
- After pushing, switch back to `main` before starting further specs.
- Archive the change once it is merged.

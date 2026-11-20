# Tasks

## 1. Branch and data export

- [x] 1.1 Create branch `chore/replace-sqlite-with-csv` off `main` and verify `git branch --show-current` prints it
- [x] 1.2 Export each `.db` in `seqa_core/data/` to CSV with `sqlite3 -list -separator ,`, using `SELECT * FROM coordinates` for genes and `SELECT * FROM cytobands WHERE name != ''` for cytobands. Write `<genome>.<source>.genes.csv` and `<genome>.cytobands.csv`. Verify with `wc -l` that the counts are 59009 (hg38 RefSeq), 45140 (hg19 RefSeq), 78733 (hg38 GENCODE), 80315 (hg19 GENCODE), 862 (hg38 cytobands) and 862 (hg19 cytobands). Also verify `grep -c '""'` finds 0 in every file and `awk -F, '$4==""'` prints nothing for either cytoband file
- [x] 1.3 Re-export each `.db` the same way into a second copy and verify `diff` against the committed CSVs is empty. Also verify every gene row has exactly 4 fields and every cytoband row exactly 5 (`awk -F, 'NF!=4'` / `'NF!=5'` prints nothing)

## 2. `reference_data` module (tests first)

- [x] 2.1 Write `seqa_core/tests/reference_data.rs` covering every spec scenario in `specs/reference-data/spec.md` for lookup behaviour: BRCA1/BRCA2/TP53 on the expected chromosomes in all four gene datasets, `GRCh37` alias, `mm10` fallback, `hg19`/`ensembl` returning the same slice as `hg19`/`refseq`, first hg38 `chr1` band, all hg19 `chr1` bands on `chr1`, `1` → empty list, `chr1_KI270762v1_alt` and `chrM` returning empty lists for hg38, every band in both builds having a non-empty name, and repeated calls returning the same slice (`std::ptr::eq`). Register it in `seqa_core/tests/mod.rs`. Verify it fails to compile (module missing)
- [x] 2.2 Add `seqa_core/src/reference_data/mod.rs` with `gene_coordinates` and `cytobands` as in design.md: `include_str!` the six CSVs, one `OnceLock` per dataset, `normalize_genome` moved over unchanged, an unrecognised source selecting RefSeq for the resolved genome, comma-split parsing that panics with dataset name and line number on a bad row. Docstrings on both public functions. Export it from `seqa_core/src/lib.rs`. Verify `cargo test -p seqa_core --test mod reference_data` passes

- [x] 2.3 Write `seqa_core/tests/reference_data_integrity.rs` (registered in `seqa_core/tests/mod.rs`) for the two integrity requirements in the spec. Genes: ≥ 40,000 rows per dataset, chromosome in `chr1`–`chr22`/`X`/`Y`/`M`, non-empty symbol, `begin ≥ 1`, `begin ≤ end`. Cytobands: exactly the 24 chromosomes per build, known stain set, `begin < end`, bands per chromosome start at 0 and tile with no gaps or overlaps. Each failing assertion names the dataset and the offending row. Verify `cargo test -p seqa_core --test mod reference_data_integrity` passes. Also temporarily delete one interior band from a cytoband CSV and one gene row's symbol, confirm the matching tests fail, then restore the files (`git diff seqa_core/data` empty)

The parser's panic paths can't be reached through the public API, because the data is embedded. They stay untested rather than making the parsers public just for tests.

## 3. Remove SQLite from `seqa_core`

- [x] 3.1 Delete `seqa_core/src/sqlite/`, `seqa_core/tests/sqlite.rs` (and its `mod` line), and `seqa_core/data/*.db`. Remove the `[features]` section (`sqlite` and `default`) and `rusqlite` from `seqa_core/Cargo.toml`, and make sure `pub mod reference_data` in `lib.rs` has no `#[cfg(feature = ...)]`. Verify `grep -rn -i sqlite seqa_core` only matches the README lines fixed in 3.2, and `cargo test -p seqa_core --no-default-features --test mod reference_data` passes
- [x] 3.2 Update the feature-flag section in `seqa_core/src/lib.rs` docs and `seqa_core/README.md:111` to describe `reference_data` (no feature flag). Verify `cargo doc -p seqa_core --no-deps` builds without warnings
- [x] 3.3 Bump `seqa_core` to 10.0.0 and verify `cargo build -p seqa_core` succeeds

## 4. Front ends

- [x] 4.1 Lambda: add a test in `seqa_lambda/src/http_handler.rs` tests for `GET /genes/cytobands/hg19/chr1` and one for an unknown chromosome returning 200 with `[]`, then switch `gene_symbols` and `cytobands` to `reference_data` and drop the `features = ["sqlite"]` entry. Verify `cargo test -p seqa_lambda` passes (including the existing `gene_coordinates_brca2_grch38` and `cytobands_chr1_grch38`)
- [x] 4.2 axum: extend `test_cytobands_route_matches_client_path` (`seqa_axum/src/main.rs:606`) to assert 200 and a non-empty JSON array. Add a test that `GET /genes/symbols/grch38/refseq/` returns 200 with a JSON array containing `BRCA2`, and confirm it fails on the current code. Then change `get_gene_symbols` to take `Path((genome, source)): Path<(String, String)>`, switch both handlers to `reference_data` (removing the DB error branches), and drop the `features = ["sqlite"]` entry. Verify `cargo test -p seqa_axum` passes
- [x] 4.3 CLI: add `seqa_cli/tests/reference_data.rs` running the built binary (`env!("CARGO_BIN_EXE_seqa")`) for `cytobands chr1`, `genes coordinates BRCA1` and `genes symbols`, checking output format against the spec. Then switch `seqa_cli/src/main.rs` to `reference_data`, remove the `GeneError` import, change `run_gene_command` to not return a `Result` from `GeneError`, update the `Genes`/`Cytobands` doc comments that say "SQLite", and drop the `features = ["sqlite"]` entry. Verify `cargo test -p seqa` passes
- [x] 4.4 Update `seqa_cli/README.md:67` to say gene data is bundled CSV. Verify `grep -n -i sqlite seqa_cli/README.md` is empty
- [x] 4.5 Patch-bump `seqa` (3.1.1), `seqa_axum` (6.0.1) and `seqa_lambda` (4.0.1). Verify `cargo build --workspace` succeeds

## 5. Data scripts

- [x] 5.1 `git mv scripts/create_gene_coordinates_db.sh scripts/create_gene_coordinates.sh` (keeps the uncommitted edits), then strip the sqlite import so it only runs `create_gene_csv.py`. Change `create_gene_csv.py` to write `<genome>/<genome>.<source>.genes.csv`. Verify with `bash -n` and by running `create_gene_csv.py` on a small gzipped GTF fixture under `scripts/` that output rows match the `chromosome,begin,end,symbol` format
- [x] 5.2 Change `download_cytobands.sh` to write `<genome>/<genome>.cytobands.csv` via `zcat | awk -F'\t' -v OFS=, '$4 != "" { $1=$1; print }'` instead of importing into SQLite. Verify with `bash -n`, and by running the conversion on a three-line sample (two named bands and one empty-name alt row) and checking that only the two named bands come out as comma-separated rows
- [x] 5.3 Update `install_all.sh` to call `create_gene_coordinates.sh`, copy the six CSVs instead of the `.db` files, and drop `sqlite3` from its dependency comment. Verify with `bash -n`
- [x] 5.4 Add `set -euo pipefail` to `get_transcript_data.sh`, `create_gene_coordinates.sh`, `download_cytobands.sh` and `install_all.sh`. In `get_transcript_data.sh`, remove the `if [[ ! -f ... ]]` download skips and delete leftover downloads before fetching. In `install_all.sh`, delete `./hg19`, `./hg38` and `./mane.bb` before running the other scripts. Verify each with `bash -n`
- [x] 5.5 Add `scripts/tests/halt_on_failure.sh`. It runs `install_all.sh` in a scratch directory under `scripts/tests/` with a `PATH` whose `wget` stub exits 1 and whose `aws` stub records any call. It asserts the run exits non-zero, `aws` was never called, and `seqa_core/data/` is unchanged. A second case pre-creates a stale `hg38/` directory and checks that it was removed. Verify the test script passes and cleans up its scratch directory
- [x] 5.6 Update `scripts/README.md`: remove `sqlite3` from dependencies and the apt line, rename the script section, replace the table schemas with the CSV column layouts, and note that `download_cytobands.sh` drops UCSC's empty-name placeholder rows. Say that the scripts always download fresh and stop on the first error. Add a "Refreshing data and publishing" section with the manual steps: `install_all.sh` → `cargo test -p seqa_core` → review and commit the CSV diff → `cargo publish -p seqa_core`. Verify `grep -n -i sqlite scripts/` is empty

## 6. Integration checks

- [x] 6.1 Run `cargo test --workspace` and verify every test passes
- [x] 6.2 Run `cargo clippy --workspace` and verify there are no warnings
- [x] 6.3 Verify `cargo tree -p seqa_core | grep -i sqlite` is empty and `grep -rn -i "sqlite\|rusqlite" --include=*.rs --include=*.toml --include=*.md .` (excluding `openspec/`, `target/`) is empty
- [x] 6.4 Run `openspec validate replace-sqlite-with-csv --strict` and verify it passes
- [x] 6.5 Remove the `libsqlite3-dev` install steps from `.github/workflows/coverage.yml` and `.github/workflows/publish.yml`. Verify `grep -rn sqlite .github` is empty
- [x] 6.6 Add a step to the `test` job in `.github/workflows/ci.yml` that runs `cargo test -p seqa_core --test reference_data --test reference_data_integrity`. Verify the YAML parses and the command passes locally

## Workflow follow-up

- Commit as `feat!: replace sqlite reference data with in-memory csv`, tag `seqa_core` v10.0.0, and check with the user before pushing.
- Archive the change after review.

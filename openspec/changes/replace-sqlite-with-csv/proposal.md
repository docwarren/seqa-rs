# Proposal

## Why

Gene coordinates and cytobands are shipped as six SQLite databases embedded in the binary. To open them, `seqa_core` writes each one to the system temp directory on first use and opens it with `rusqlite` (`seqa_core/src/sqlite/mod.rs:39-57`). That adds a bundled C dependency, a temp-file write on every cold start (awkward on Lambda), and a SQL layer that only ever runs `SELECT *` and one `WHERE chromosome = ?`. The data is small (≤ 3 MB per file) and read-only, so plain CSV parsed into memory is simpler and needs no filesystem.

## What Changes

- Replace the six embedded `.db` files in `seqa_core/data/` with CSV files holding the same rows (headerless, `chromosome,begin,end,symbol` for genes; `chromosome,begin,end,name,stain` for cytobands).
- Drop cytoband rows with an empty band name. These are UCSC's placeholder rows for hg38 alt, fix, unplaced and random sequences and `chrM` (687 rows, none in hg19). The data scripts filter them out too.
- Embed the CSVs in the binary and parse each dataset into memory on first use, caching it for the life of the process.
- **BREAKING**: remove the `seqa_core::sqlite` module, the `sqlite` cargo feature and the `rusqlite` dependency. Add a `seqa_core::reference_data` module in its place. It is always compiled in, not gated behind a feature, so every crate that depends on `seqa_core` gets the reference data. `GeneError` (with its `rusqlite`-typed `DatabaseError` variant) goes away. `seqa_core` goes from 9.0.1 to 10.0.0.
- Update `seqa_cli`, `seqa_axum` and `seqa_lambda` to call `reference_data`. Their HTTP routes, CLI commands and output stay the same, apart from the axum gene symbols fix below.
- **Fix:** an unrecognised gene source now returns RefSeq for the requested genome build (`hg19`/`ensembl` → hg19 RefSeq). Today it returns hg38 RefSeq whatever the genome.
- **Fix:** the axum route `GET /genes/symbols/{genome}/{source}/` reads both path parameters in one extractor. It currently uses two separate single-value extractors and fails on every request.
- Change the data scripts to produce the CSVs instead of SQLite databases, and drop `sqlite3` from their dependencies.
- Make the data scripts download from scratch on every run and stop on the first failing command. Today they reuse files already on disk and carry on after errors.
- Add data integrity tests to `seqa_core`'s test suite so freshly regenerated CSVs are checked before a manual `cargo publish`. They check row floors, valid chromosomes, coordinates, stains, cytoband tiling and key genes.
- Update the READMEs that mention SQLite.

## Capabilities

### New Capabilities
- `reference-data`: lookup of gene coordinates (by genome build and source) and cytobands (by genome build and chromosome) from data bundled with `seqa_core`.

### Modified Capabilities
<!-- None: openspec/specs/ has no existing capabilities. -->

## Impact

- **Code**: `seqa_core/src/sqlite/` (removed), new `seqa_core/src/reference_data/`, `seqa_core/src/lib.rs`, `seqa_cli/src/main.rs`, `seqa_axum/src/main.rs`, `seqa_lambda/src/http_handler.rs`.
- **Tests**: `seqa_core/tests/sqlite.rs` and the unit tests in `seqa_core/src/sqlite/mod.rs` are replaced by tests in `seqa_core/tests/`.
- **Data**: `seqa_core/data/*.db` replaced by `*.csv`.
- **Dependencies**: `rusqlite` removed from `seqa_core`; the `features = ["sqlite"]` entries are removed from the `seqa_cli`, `seqa_axum` and `seqa_lambda` manifests.
- **Scripts/docs**: `scripts/create_gene_coordinates_db.sh`, `scripts/download_cytobands.sh`, `scripts/get_transcript_data.sh`, `scripts/install_all.sh`, `scripts/README.md`, `seqa_core/README.md`, `seqa_cli/README.md`.
- **Versions**: `seqa_core` 10.0.0 (major). The `seqa_cli`, `seqa_axum` and `seqa_lambda` versions are bumped only where their own public surface changes (it does not), so a patch bump for the dependency change.

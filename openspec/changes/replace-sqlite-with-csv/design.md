# Design

## Context

See proposal.md - Why. Current state:

- `seqa_core/src/sqlite/mod.rs:23-28` embeds six `.db` files with `include_bytes!`. `open_embedded` writes each to `$TMPDIR/seqa-<version>-<name>` and opens it read-only.
- `seqa_core/src/sqlite/genes.rs:36` runs `SELECT * FROM coordinates` and `:57` runs `SELECT * FROM cytobands WHERE chromosome = ?1`. No other SQL exists.
- Callers: `seqa_cli/src/main.rs:180-216`, `seqa_axum/src/main.rs:249-268`, `seqa_lambda/src/http_handler.rs:112-131`. Each opens a connection per request or command.
- Row counts: hg38 RefSeq 59009, hg19 RefSeq 45140, hg38 GENCODE 78733, hg19 GENCODE 80315, hg38 cytobands 1549, hg19 cytobands 862.
- No gene symbol contains a comma. 687 hg38 cytobands have an empty `name`. They are UCSC whole-sequence placeholders for `*_alt` (351), `*_fix` (166), `chrUn_*` (127), `*_random` (42) and `chrM` (1). hg19 has none. This change drops them, leaving 862 hg38 bands.
- `GeneCoordinate` and `Cytoband` already exist in `seqa_core/src/models/` and are serialised to JSON by the HTTP front ends.
- None of the data scripts use `set -e`, so `install_all.sh` carries on after a failed step and still copies and uploads whatever exists. `get_transcript_data.sh` skips a download when the file is already on disk (`if [[ ! -f ${gencode_local} ]]`).
- In the current data, every gene chromosome is `chr1`–`chr22`, `chrX`, `chrY` or `chrM`, every `begin` ≥ 1 and < `end`, and BRCA1, BRCA2 and TP53 are in all four datasets. The named cytobands cover exactly 24 chromosomes per build, use 8 stain values, and tile each chromosome from 0 with no gaps.
- Publishing is manual: regenerate the data by hand, then `cargo publish`.

## Goals / Non-Goals

**Goals:**
- For the 10.0.0 release, the same rows (minus the cytoband placeholders), order and JSON/CLI output as 9.0.1.
- Future data refreshes are checked by integrity tests rather than compared to a fixed snapshot.
- No temp files, no database engine and no new dependencies.

**Non-Goals:**
- Indexed or range queries over genes (the API still returns whole lists).
- Changing the HTTP routes or CLI commands.
- Automating the release. No release script, `build.rs` or CI publish step. Data refresh and `cargo publish` stay manual, separate steps.

## Decisions

### Module and API

New public module `seqa_core::reference_data` replaces `seqa_core::sqlite`:

```rust
/// Every gene in the bundled dataset for `genome` and `source`.
pub fn gene_coordinates(genome: &str, source: &str) -> &'static [GeneCoordinate];

/// Every bundled cytoband for `genome` on `chromosome`.
pub fn cytobands(genome: &str, chromosome: &str) -> Vec<Cytoband>;
```

`gene_coordinates` returns a borrowed slice of the cached data, so callers don't copy 80k rows per request. `cytobands` filters and clones the few dozen matching bands. The existing `normalize_genome` logic moves over unchanged. The `sqlite` feature and module are removed, not deprecated, because the crate is already taking a major version bump. `reference_data` has no feature flag. Without SQLite it adds no dependencies, only about 8 MB of embedded CSV, so there is no reason to make it optional.

### Infallible lookups

The data is compiled into the binary, so a parse failure can only come from a bad data file committed to the repo, never from user input or runtime conditions. The functions therefore return values, not `Result`, and a malformed row panics with the dataset name and line number. Tests load all six datasets, so a bad file fails CI before it ships. `GeneError` is removed. Front ends lose their "error connecting to DB" branches.

*Alternative:* keep a `Result` and an error type. Rejected: it makes every caller handle an error that a passing test suite rules out.

### Lazy, once-per-process loading

Each dataset has its own `static OnceLock<Vec<T>>`, filled on first request. A Lambda cold start or CLI run only parses what it uses, and later requests in a long-running axum process reuse the data.

*Alternative:* parse everything at server startup into shared state. Rejected: that slows every Lambda cold start by all six datasets and adds state plumbing to the axum and Lambda handlers.

### CSV format and parsing

Headerless, comma-separated, one row per line, no quoting:
- Genes: `chromosome,begin,end,symbol` (what `scripts/create_gene_csv.py` already writes).
- Cytobands: `chromosome,begin,end,name,stain`. Rows with an empty name are left out when the CSV is made, so every row has a name.

Rows are parsed by splitting on `,` and checking the field count. No new dependency.

*Alternative:* the `csv` crate with serde. Rejected: the format has no quoting or escaping, so a crate adds a dependency for no gain.

Files live at `seqa_core/data/<genome>.<source>.genes.csv` and `seqa_core/data/<genome>.cytobands.csv`, embedded with `include_str!`. They are uncompressed. Total size is about the same as the `.db` files, and gzip would add a decompression step on first use.

### Producing the CSVs

The first set of CSVs is exported from the shipped `.db` files rather than rebuilt from source, so the data matches 9.0.1 exactly:

```bash
sqlite3 -list -separator , <db> "SELECT * FROM coordinates" > <genes csv>
sqlite3 -list -separator , <db> "SELECT * FROM cytobands WHERE name != ''" > <cytobands csv>
```

`-list` is used instead of `-csv` so fields are never quoted. Before the `.db` files are deleted, a row count and a full diff against a second export prove they match.

From then on, the scripts produce the CSVs directly:
- `create_gene_coordinates_db.sh` is renamed to `create_gene_coordinates.sh` and only runs `create_gene_csv.py`, writing `<genome>/<genome>.<source>.genes.csv`.
- `download_cytobands.sh` converts UCSC's tab-separated `cytoBand.txt` to `<genome>/<genome>.cytobands.csv`, dropping empty-name rows: `awk -F'\t' -v OFS=, '$4 != "" { $1=$1; print }'`.
- `install_all.sh` copies the CSVs instead of the `.db` files.

### Bug fixes folded in

- **Unknown source fallback.** `seqa_core/src/sqlite/mod.rs:65` falls back to hg38 RefSeq for any unrecognised source, even when the genome is hg19. In `reference_data`, dataset selection matches on the resolved genome when the source is not `gencode`, so an unrecognised source gets that genome's RefSeq.
- **axum gene symbols extractor.** `seqa_axum/src/main.rs:249-251` declares `Path(genome): Path<String>` and `Path(source): Path<String>` for a two-parameter route. Axum can't fill a single `String` from two parameters, so the request fails. The handler takes `Path((genome, source)): Path<(String, String)>`, as `get_cytobands` already does.

### Data integrity tests

The integrity tests live in `seqa_core/tests/reference_data_integrity.rs` and run against the embedded data through the public API, as part of the normal `cargo test`. Running the test suite before `cargo publish` is therefore what gates a data refresh. The rules are in the spec. They are invariants, not snapshot values:
- **Row floors** (40,000 genes per dataset). The smallest current dataset has 45,140. A floor catches truncated downloads without failing on normal growth between annotation releases.
- **Fixed sets** for chromosomes and stains.
- **Tiling check** for cytobands, which catches dropped or duplicated rows.
- **Key genes** (BRCA1, BRCA2, TP53) and their chromosomes.

Exact counts and exact BRCA1 coordinates are not in the permanent tests, because they change with every GENCODE or RefSeq release. Parity with 9.0.1 is checked once, while migrating (tasks 1.2–1.3).

*Alternative:* a standalone validation script. Rejected: the Rust tests already parse the data exactly as the library does, and they run with every `cargo test`.

### Script failure handling and clean downloads

Every data script starts with `set -euo pipefail`. `install_all.sh` deletes `./hg19`, `./hg38` and `./mane.bb` in its working directory before starting. `get_transcript_data.sh` drops its `if [[ ! -f ... ]]` skips and deletes any leftover downloads before fetching. Because `install_all.sh` copies to `seqa_core/data/` and uploads to S3 only at the end, any earlier failure leaves both untouched.

## Risks / Trade-offs

- [`pipefail` makes `grep -v '^N'` in `get_transcript_data.sh` fail the run if it filters out every line] → That would mean a broken download, so halting is the right result.
- [A clean run re-downloads several GB every time] → Accepted. It's a manual, occasional step.

- [A future GTF has a gene name containing a comma, which breaks the field count] → The parser panics with the dataset and line, and the load-every-dataset test fails in CI. `create_gene_csv.py` can then be changed to reject or escape it.
- [Breaking API for external `seqa_core` users] → Major version bump to 10.0.0. The README documents the replacement functions.
- [Uncommitted edits to `scripts/create_gene_coordinates_db.sh` in the working tree] → Rename with `git mv` so the edits are kept, then rework the script.
- [`gene_coordinates` hands out `&'static` data] → The data is read-only and lives for the process anyway, so there is no lifetime cost.

## Migration Plan

1. Export the CSVs, verify them against the `.db` files, then delete the `.db` files.
2. Land `reference_data`, switch the front ends, remove `rusqlite`.
3. Release `seqa_core` 10.0.0. Front ends pick it up with a patch bump.

Later data refreshes, done by hand and documented in `scripts/README.md`:
1. Run `install_all.sh`.
2. Run `cargo test -p seqa_core`.
3. Review the CSV diff and commit it.
4. Run `cargo publish -p seqa_core`.

Rollback: revert the commit. The `.db` files come back from git history.

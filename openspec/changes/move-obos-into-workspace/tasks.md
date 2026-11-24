# Tasks

## 1. Pre-flight

- [x] 1.1 Branch off `main` in seqa-rs as `chore/move-obos-into-workspace`
- [x] 1.2 In `/home/drew/dev/seqa-obos`, confirm `git status` is clean and `main` has no commits ahead of `origin/main`

## 2. Copy the crate

- [x] 2.1 Copy `src/`, `tests/` (with `tests/fixtures/`), `README.md` and `Cargo.toml` from `/home/drew/dev/seqa-obos` into `seqa_obos/` in seqa-rs; do not copy `target/`, `.git/`, `.gitignore`, `Cargo.lock` or `openspec/`
- [x] 2.2 In `seqa_obos/Cargo.toml`, rename the package to `seqa_obos` and add `authors`, `rust-version`, `description`, `license`, `repository` and `homepage` matching `seqa_api/Cargo.toml`
- [x] 2.3 Add `"seqa_obos"` to `members` in the root `Cargo.toml`

## 3. License headers

- [x] 3.1 Add the Apache 2.0 header from `license_headers.py` to the top of every `.rs` file in `seqa_obos/src` and `seqa_obos/tests` (before any `//!` doc comments)

## 4. Upgrade object_store

- [x] 4.1 Read the object_store 0.13.0 and 0.14.0 changelogs and note the changes that affect `seqa_core/src/stores/` and `seqa_api/src/notes/mod.rs`
- [x] 4.2 Set `object_store` to `0.14.2` in `seqa_core`, `seqa_api`, `seqa_axum` and `seqa_lambda` `Cargo.toml`, keeping each crate's feature list
- [x] 4.3 Fix compile errors in `seqa_core` caused by the upgrade
- [x] 4.4 Fix compile errors in `seqa_api` (including `seqa_api/tests/common/s3_cleanup.rs`) caused by the upgrade
- [x] 4.5 Confirm `cargo tree -i object_store` shows only 0.14.2

## 5. Version bump

- [x] 5.1 Bump `seqa_core` to `11.0.0` in `seqa_core/Cargo.toml`
- [x] 5.2 Change `seqa_core`'s version requirement in `seqa_api/Cargo.toml` to `"11.0.0"`

## 6. Docs

- [x] 6.1 In `seqa_obos/README.md`, change the heading and crate references from `seqa-obos` to `seqa_obos`, and `cargo install --path .` to `cargo install --path seqa_obos` (or note it runs from the crate directory)
- [x] 6.2 Add a `seqa_obos` row to the crate table in the root `README.md`

## 7. CI

- [x] 7.1 Add a step to the `test` job in `.github/workflows/ci.yml` that runs `cargo test -p seqa_obos --features cli`

## 8. Verify

- [x] 8.1 `cargo build --workspace` succeeds and `Cargo.lock` is updated
- [x] 8.2 `cargo test -p seqa_obos --features cli` passes from the workspace root
- [x] 8.3 The CI test commands in `.github/workflows/ci.yml` pass locally (`seqa_core` local and reference-data tests, `seqa_axum` tests), plus `cargo test -p seqa_api`
- [x] 8.4 Run the cloud-backed `seqa_core` tests locally if credentials are available; otherwise note they were not run
- [x] 8.5 `cargo clippy --workspace` and `cargo clippy -p seqa_obos --features cli` report no warnings (flag, don't fix, any pre-existing warnings in copied code)
- [x] 8.6 Commit the move as `chore: move seqa_obos into workspace` and the upgrade as `feat!: upgrade object_store to 0.14` (separate commits)

## 9. Retire the old repo

- [x] 9.1 Delete `/home/drew/dev/seqa-obos/.git` (leave the other files and the GitHub repo untouched)

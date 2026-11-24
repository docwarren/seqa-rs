# Design

## Context

`/home/drew/dev/seqa-obos` is a standalone crate (package `seqa-obos` 0.1.0, edition 2024) with a library, an `obos` binary behind the `cli` feature, and integration tests under `tests/` that use fixtures in `tests/fixtures/` (`mini.obo`, `mini2.obo`, and the 10.8 MB `hp.obo`). Some tests are `#[ignore]`d (S3, needing credentials) and some need `--features cli` (they run the `obos` binary). Its `Cargo.toml` has no authors/license/repository metadata, unlike the seqa-rs crates. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- `seqa_obos` builds, passes clippy and passes its tests from the seqa-rs workspace root.
- The crate looks like its siblings: underscore name, same `[package]` metadata fields.

**Non-Goals:**
- No changes to `seqa_obos` source code or tests beyond what the move forces.
- No shared `[workspace.dependencies]`; each crate keeps its own `object_store` entry and feature list, with only the version changed.
- No license headers added to files outside `seqa_obos`.
- No changes to the `publish.yml` workflow; `seqa_obos` is not published or released.
- No version bumps other than `seqa_core`'s. Crates that depend on `seqa_core` through a path only (`seqa_axum`, `seqa_cli`, `seqa_lambda`) need no change.
- Nothing done to the `docwarren/seqa-obos` GitHub repo or the files left in `/home/drew/dev/seqa-obos`.

## Decisions

- **Plain copy, no history.** The source repo has two commits; carrying them over with `git subtree` or `filter-repo` adds steps for little value. Its history stays readable on GitHub.
- **Directory and package both named `seqa_obos`.** Matches `seqa_api`, `seqa_axum`, etc. The lib target name is unchanged (`seqa_obos`), so `use seqa_obos::...` in `src/main.rs`, tests and README examples all keep working.
- **Upgrade every crate to `object_store = "0.14.2"`**, the latest release and the one `seqa_obos` already uses. Compile errors from API changes in 0.13/0.14 are fixed in place, without changing `seqa_core`'s own function signatures beyond the `object_store` types they already expose. `seqa_axum` and `seqa_lambda` list `object_store` in `Cargo.toml` without using it in code; they are bumped too, so the workspace resolves only one version.
- **`seqa_core` 10.0.0 → 11.0.0.** Its public API returns `ObjectMeta` (`list_objects`) and `HttpStore` (`get_http_store`), and `StoreError` wraps `object_store::Error`. Those are now 0.14 types, which code built against 0.12 cannot accept. `seqa_api` requires `version = "10.0.0"` and is updated to `"11.0.0"`.
- **License headers via the existing `license_headers.py` header text.** The header is added to every `.rs` file under `seqa_obos/src` and `seqa_obos/tests`. The header string is copied exactly; the script itself is not changed, since it only walks `seqa_axum`.
- **Keep the `obos` binary name and `cli` feature.** Changing them would alter the CLI's user-facing behavior.
- **Metadata copied from `seqa_api/Cargo.toml`**, with `version = "0.1.0"` kept and a description taken from the first line of the crate README.
- **CI runs the non-ignored tests with the CLI enabled**: `cargo test -p seqa_obos --features cli`. Without `--features cli`, the CLI tests are skipped, so this is the one command that covers everything that runs offline.

## Risks / Trade-offs

- [`object_store` 0.13/0.14 API changes (for example in the custom `client::HttpClient` code in `seqa_core/src/stores/store.rs`) may need more than renames] → Check the object_store changelog for 0.13.0 and 0.14.0 before editing; cloud-backed tests (S3/Azure/GCS) only run in the coverage workflow, so run them locally with credentials where possible.
- [Breaking `seqa_core` release for external users] → Major version bump signals it.
- [10.8 MB `hp.obo` fixture added to seqa-rs history permanently] → Accepted; `hpo_integration.rs` needs it offline.
- [`cargo clippy --workspace` does not enable `cli`, so `src/main.rs` is not linted by the standard gate] → Also run `cargo clippy -p seqa_obos --features cli` during implementation.
- [Deleting `/home/drew/dev/seqa-obos/.git` loses any unpushed work] → Check `git status` and that `main` matches `origin/main` before deleting.

## Migration Plan

1. Copy and wire up the crate in seqa-rs on a feature branch; verify build, clippy and tests.
2. Only after that, delete `/home/drew/dev/seqa-obos/.git`.
Rollback: revert the seqa-rs branch; the source repo can be re-cloned from GitHub.

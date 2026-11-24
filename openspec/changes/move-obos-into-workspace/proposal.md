# Proposal

## Why

`seqa-obos` lives in its own repo at `/home/drew/dev/seqa-obos`, separate from the other seqa crates, and its package name (`seqa-obos`) uses a hyphen where every seqa-rs crate uses an underscore (`seqa_api`, `seqa_axum`, ...). Moving it into the seqa-rs workspace keeps all seqa Rust crates in one place, built, linted and tested together.

## What Changes

- Copy the `seqa-obos` source (`src/`, `tests/` including fixtures, `README.md`) into a new `seqa_obos/` crate directory in seqa-rs, as a plain copy with no git history carried over.
- Rename the package from `seqa-obos` to `seqa_obos`. The library import path is already `seqa_obos`, so no code using it changes. The `obos` binary name is unchanged.
- Add `seqa_obos` to the workspace `members` in the root `Cargo.toml`.
- Give `seqa_obos/Cargo.toml` the same package metadata the other crates carry (authors, rust-version, description, license, repository, homepage).
- Drop `seqa-obos`'s own `Cargo.lock`, `.gitignore` and empty `openspec/` folder; the workspace lockfile and root `.gitignore` cover them.
- Add `seqa_obos` to the crate table in the root `README.md` and fix the crate name in `seqa_obos/README.md`.
- Add the Apache 2.0 license header used by the other crates to every `.rs` file in `seqa_obos`.
- Upgrade `object_store` from 0.12.4 to 0.14.2 in `seqa_core`, `seqa_api`, `seqa_axum` and `seqa_lambda`, so the whole workspace uses the version `seqa_obos` already uses.
- **BREAKING** (for `seqa_core` users outside the workspace): `seqa_core` returns `object_store` types from its public API, so it goes from 10.0.0 to 11.0.0. `seqa_api`'s `seqa_core` version requirement is updated to match.
- Add a CI step that runs the `seqa_obos` tests.
- Delete `/home/drew/dev/seqa-obos/.git`. The rest of that directory and the `docwarren/seqa-obos` GitHub repo are left as they are.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a repository restructuring: `seqa_obos`'s library API and `obos` CLI behave exactly as before, so `skip_specs: true` is set.

## Impact

- New `seqa_obos/` crate in the seqa-rs workspace; workspace `Cargo.toml`, `Cargo.lock`, root `README.md` and `.github/workflows/ci.yml` change.
- `seqa_obos` brings `fastobo` into the workspace.
- `object_store` 0.14 changes the API in places, so code in `seqa_core/src/stores/` and `seqa_api/src/notes/mod.rs` may need updates to compile.
- `seqa_core` 11.0.0: anything outside the workspace that uses `ObjectMeta`, `HttpStore` or `StoreError::ObjectStoreCreationError` from `seqa_core` must move to `object_store` 0.14.
- A 10.8 MB fixture (`tests/fixtures/hp.obo`) is committed to seqa-rs.
- `/home/drew/dev/seqa-obos` stops being a git repo.

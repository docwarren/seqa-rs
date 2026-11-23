# Tasks

**Test placement:**
- `seqa_api` tests go in `seqa_api/tests/` (groups 1 and 2 first put them in `seqa_core/tests/`; group 5 moves them).
- `seqa_axum` and `seqa_lambda` are binary crates, so their route tests go in the existing inline `#[cfg(test)]` modules (`seqa_axum/src/main.rs`, `seqa_lambda/src/http_handler.rs`). This was agreed with the user.
- Round-trip note tests write under `s3://com.gmail.docarw/test_data/notes/`.

## 0. Branch

- [x] 0.1 Branch `feat/endpoint-parity` off `main` and verify with `git branch --show-current`

## 1. seqa_core shared API types

- [x] 1.1 Write `seqa_core/tests/search_request.rs`. Cover JSON deserialization (snake_case fields, `genome` ignored, a string `coordinates` rejected) and every `to_search_options` rule in the `http-api` "Search request" and "Search request validation" requirements. Verify the tests fail.
- [x] 1.2 Implement `SearchRequest`, `SearchRequestError` and `to_search_options` in `seqa_core/src/api/search_request.rs`, with docstrings. Verify the 1.1 tests pass.
- [x] 1.3 Write tests for `SearchResponse::from_result`: the header is filled only when `include_header` or `header_only` is set, and there is one `lines` entry per coordinate. Verify they fail.
- [x] 1.4 Implement `SearchResponse` in `seqa_core/src/api/search_response.rs`. Verify the 1.3 tests pass.
- [x] 1.5 Move `ListDirectoryRequest` to `seqa_core/src/api/list_directory_request.rs`, and `is_local_uri` to `seqa_core::stores`. Write tests in `seqa_core/tests/stores_helpers.rs` (`file:`, scheme-less, `s3://`, mixed case, leading whitespace). Verify they pass.

## 2. seqa_core notes

- [x] 2.1 Add the `base64` dependency to `seqa_core/Cargo.toml`. Verify `cargo build -p seqa_core`.
- [x] 2.2 Write `seqa_core/tests/notes.rs` covering every scenario of "Note paths must be inside a notes directory". Verify the tests fail.
- [x] 2.3 Implement `NotesError`, the request/response types and `validate_note_path` in `seqa_core/src/notes/`, and register `pub mod notes`. Verify the 2.2 tests pass.
- [x] 2.4 Add `read_note`/`save_note` tests: S3 round-trip, overwrite, missing → `NotFound`, bad base64 → `InvalidData` with nothing written, bad path → `InvalidPath` with nothing written, and a local `file://` round-trip under a `notes` directory inside the workspace test data dir. Verify the tests fail.
- [x] 2.5 Implement `read_note` and `save_note`, with docstrings. Verify the 2.4 tests pass.

## 3. seqa_axum

- [x] 3.1 Add tests to the axum test module for the `http-api` requirements axum doesn't meet yet:
  - JSON error bodies, including on the fallback 404;
  - `POST /files` → 404;
  - trailing slash optional on gene symbols and cytobands;
  - `index_path` and `output_format` honored;
  - unknown `output_format` → 400;
  - `/notes/read` and `/notes/save`: 200, 204 then read back, 404, 400 for an invalid or local path, 400 for bad base64, 400 for a malformed body.

  Verify the new tests fail.
- [x] 3.2 Replace the axum `SearchRequest`, `AxumSearchResult`, `ListDirectoryRequest`, extension list and `is_local_uri` with the core types and functions. Delete `seqa_axum/src/search/models.rs` if it is left empty. Verify the existing search and listing tests still pass.
- [x] 3.3 Remove the `/files` route, `list_dir` and `FileEntry`.
- [x] 3.4 Change `ApiError` and the fallback to JSON `{"error"}`, and add the notes handlers and error mapping.
- [x] 3.5 Add `NormalizePathLayer::trim_trailing_slash()` (enable the tower-http `normalize-path` feature), and register gene symbols without the trailing slash.
- [x] 3.6 Verify all 3.1 tests pass (`cargo test -p seqa_axum`).

## 4. seqa_lambda

- [x] 4.1 Update the existing Lambda tests to the new paths (`/search`, `/cytobands/...`). Add tests for:
  - unknown POST → 404 with a JSON error;
  - `/files/list`: 200 on an S3 prefix, 400 for `file:///etc` and `/etc`, 400 for a malformed body;
  - `/files/buckets/{backend}`: 400 for `nonsense` and `local`;
  - `/files/formats` → 200;
  - trailing slash on gene symbols and cytobands;
  - search with `index_path` and `output_format`, and with an unknown `output_format` → 400;
  - the same `/notes/read` and `/notes/save` cases as 3.1.

  Verify the new tests fail.
- [x] 4.2 Replace Lambda's `SearchRequest`, `SearchResultJson` and the inline options building with the core types. Remove the `genome` handling.
- [x] 4.3 Rewrite `function_handler` routing to match the explicit route set, with a final JSON 404, and add the listing, bucket, format and notes handlers. Return 204 with `Body::Empty` from save.
- [x] 4.4 Verify all 4.1 tests pass (`cargo test -p seqa_lambda`).
- [x] 4.5 Rewrite the routes table and request/response sections of `seqa_lambda/README.md` to match the `http-api` spec, and check that the documented bodies match the tests.

## 5. Move shared code into seqa_api

Groups 1 and 2 were first implemented inside `seqa_core`. The user then decided that `seqa_core` stays a search engine, so this group moves that work into a new crate. The tasks above stay ticked as a record of what was built.

- [x] 5.1 Create the `seqa_api` library crate (version 0.1.0, edition 2024, license and repository fields matching `seqa_core`), and add it to the workspace `members`. Dependencies: `seqa_core` (path plus `version = "10.0.0"`), `serde`, `thiserror`, `url`, `base64`, and `object_store` at the same version as `seqa_core`. Verify `cargo build -p seqa_api`.
- [x] 5.2 Move `search_request.rs`, `search_response.rs`, `list_directory_request.rs`, the `notes` module, `is_local_uri`, `reject_local_uri` and `LocalUriError` from `seqa_core` into `seqa_api`, and their tests from `seqa_core/tests/` into `seqa_api/tests/`. The tests are `search_request.rs`, `search_response.rs`, `notes.rs`, `notes_integration.rs` and `stores_helpers.rs`; rename the last one to `local_uri.rs`. Keep the docstrings. Verify `cargo test -p seqa_api` passes.
- [x] 5.3 Revert `seqa_core` to `main`: `git diff main -- seqa_core` must be empty (version 10.0.0, no `base64`, `stores/error.rs` and `stores/mod.rs` unchanged). Verify `cargo test -p seqa_core` passes.
- [x] 5.4 Point `seqa_axum` and `seqa_lambda` at `seqa_api` (path dependency) for every moved item, and remove any `seqa_core` imports they no longer need. Point the shared parity table in `tests/http_parity.rs` at `seqa_api` as well. Verify `cargo test -p seqa_axum -p seqa_lambda` builds.

## 6. Spec updates from integration testing

- [x] 6.1 Lambda JSON-only bodies: the failing tests `parity_form_encoded_save_is_not_written` and the form-encoded cases in `parity_table` are already the red step. Replace `parse_payload`'s use of `RequestPayloadExt::payload` with a content-type check (`application/json`, parameters allowed) followed by `serde_json::from_slice`. Verify those tests pass and that nothing is written to S3.
- [x] 6.2 Change the parity table's unknown genome and unknown source cases to expect the hg38 and RefSeq fallback bodies, per `http-api` "Gene symbols" and "Cytobands". Verify they pass on both servers.
- [x] 6.3 Change the parity table's `HEAD` and `OPTIONS` cases:
  - `HEAD /files/formats` expects 200 with an empty body on axum and the JSON 404 on Lambda.
  - `OPTIONS` preflight from `http://localhost:5173` expects 200 with the allow-origin header on axum.

  Remove `OPTIONS /search` without preflight headers from the not-found list. Verify `parity_table` passes on both servers.
- [x] 6.4 Delete the two leftover objects written by the earlier failing run, `s3://com.gmail.docarw/test_data/notes/parity_never_written_form.txt` and `.../lambda_parity_never_written_form.txt`, with the user's go-ahead.

## 7. Release checks

- [x] 7.1 Set versions: `seqa_api` 0.1.0, `seqa_core` 10.0.0 (unchanged), `seqa_axum` 7.0.0, `seqa_lambda` 5.0.0. Verify `cargo build --workspace`.
- [x] 7.2 Run `cargo test --workspace` and verify that all tests pass.
- [x] 7.3 Run `cargo clippy --workspace` and verify there are no warnings.
- [x] 7.4 Run `cargo publish -p seqa_api --dry-run` and verify it packages. Do not publish.

## Workflow follow-up

- Check with the user before pushing.
- Publish `seqa_api` 0.1.0 to crates.io (with the user's go-ahead) before applying the seqa-mono `endpoint-parity` change.

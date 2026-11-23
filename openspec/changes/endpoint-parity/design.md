# Design

## Context

See proposal.md (Why) and the `http-api` and `notes-storage` specs for the contract. Current state that shapes the approach:

- **Duplicated search types.** `seqa_axum` and `seqa_lambda` each define a `SearchRequest` and a search response struct (`seqa_axum/src/search/models.rs`, `seqa_axum/src/main.rs:59`, `seqa_lambda/src/http_handler.rs:23-39`). Each server also builds `SearchOptions` its own way:
  - axum checks extensions against a hard-coded list (`seqa_axum/src/main.rs:107`);
  - Lambda applies `index_path` and `output_format`.
- **Local-path check.** `is_local_uri` is private to axum (`seqa_axum/src/main.rs:290`).
- **Lambda routing.** `seqa_lambda` matches `(method, trimmed path segments)` (`seqa_lambda/src/http_handler.rs:47`), with `(POST, _)` → search. Because the segments come from `trim_matches('/')`, a trailing slash is already optional there.
- **axum trailing slash.** axum 0.8 treats `/a` and `/a/` as different routes. Gene symbols is currently registered only with the trailing slash (`seqa_axum/src/main.rs:346`).
- **`set_output_format` fallback.** `SearchOptions::set_output_format` falls back to VCF for unknown names (`seqa_core/src/api/search_options.rs:164`).
- **URL normalization.** `Url::parse` normalizes `..` segments.
- **Missing objects.** `StoreError` has no not-found variant. A missing object arrives as `StoreError::ObjectStoreCreationError(object_store::Error::NotFound { .. })`.
- **Tauri depends on published crates.** seqa-mono's Tauri app depends on `seqa_core` from crates.io (`10.0.0`). So `seqa_api` has to be published before the paired change can use it.
- **Form-encoded bodies.** `lambda_http`'s `RequestPayloadExt::payload` deserializes `application/x-www-form-urlencoded` as well as JSON. Browsers send form posts cross-origin without a CORS preflight.

## Goals / Non-Goals

**Goals:**
- Every request shape, validation rule and request→options conversion lives in one shared crate, `seqa_api`. axum, Lambda and Tauri only do transport (HTTP status mapping or Tauri errors).
- `seqa_core` stays a search engine. It gains no API, notes or server-policy code, and its version does not change.
- The two servers differ only in their framework code. axum keeps its index/header cache, which is a performance detail and doesn't change behavior.

**Non-Goals:**
- Changing `seqa_core` in any way, including `StoreService` and the search engine.
- Changing `SearchOptions::set_output_format`'s VCF fallback. Unknown names are rejected before it is called.

## Decisions

### New workspace crate `seqa_api`
A library crate at `seqa_api/`, added to the workspace `members`. It depends on `seqa_core` (`path = "../seqa_core"`, `version = "10.0.0"`, so it can be published), plus `serde`, `thiserror`, `url`, `object_store` and `base64`. axum and Lambda depend on it by path, and Tauri depends on it from crates.io. Its tests go in `seqa_api/tests/`.

*Alternative considered:* putting these types in `seqa_core`, which was the first implementation. Rejected by the user: `seqa_core` is the search engine and should not own API shapes, notes or server policy.

### Shared API types in `seqa_api`
Files under `seqa_api/src/`. All are serde types with the field names from the specs (`with_header` snake_case, as the UI sends today):

- `search_request.rs`:
  - `SearchRequest { path, coordinates, with_header, only_header, index_path, output_format }`.
  - `SearchRequest::to_search_options(&self) -> Result<SearchOptions, SearchRequestError>`. It rejects empty coordinates (`No coordinates`), unparseable coordinates, unknown `output_format` names (checked with `OutputFormat::get_enum` before `set_output_format`), and a resolved format of `OutputFormat::STRING` (the extension was not recognized). This replaces axum's hard-coded extension list, which wrongly rejected files whose format comes from `output_format`.
- `search_response.rs`: `SearchResponse { coordinates, header, lines }` and `SearchResponse::from_result(result, &options)`. It fills `header` only when `include_header` or `header_only` is set.
- `list_directory_request.rs`: `ListDirectoryRequest { uri }`, moved from axum.
- Note request/response types are in `seqa_api::notes` (below).

*Alternative considered:* keep the types in each server and test that they match. Rejected because two copies drift, which is how the two servers came to differ.

### `seqa_api::notes`
`seqa_api/src/notes/mod.rs` and `notes_error.rs`:
- `ReadNoteRequest { path }`, `ReadNoteResponse { data }`, `SaveNoteRequest { path, data }`.
- `validate_note_path(path) -> Result<(), NotesError>`:
  - parse with `Url`;
  - the scheme must be one of `file`, `s3`, `gs`, `az`;
  - the normalized path segments must have a `notes` segment before a non-empty last segment. For `az` URIs the first segment is the container and is skipped.
- `read_note(store, path) -> Result<ReadNoteResponse, NotesError>`: validate, then `get_object`, then base64-encode (`base64::engine::general_purpose::STANDARD`).
- `save_note(store, request) -> Result<(), NotesError>`: validate and decode before any store call, then `put_object`.
- `NotesError`: `InvalidPath`, `InvalidData`, `NotFound` (mapped from `object_store::Error::NotFound`), `Store`.

`seqa_api::notes` allows `file://` because the Tauri app keeps notes locally. The servers add the local check (next decision). This was agreed: the shared notes code enforces only the notes rule.

### Local-URI guard
Move `is_local_uri` to `seqa_api` as a public function with a docstring. It decides with `Url::parse`, the parser the store uses: a URI is local when its scheme is `file` or it does not parse. A string prefix check is not enough, because `Url::parse` strips tabs, newlines and leading control characters (`fi\tle:///etc` parses as `file:///etc`). `reject_local_uri` wraps it and returns `LocalUriError` (defined in `seqa_api`, not in `seqa_core`'s `StoreError`), which carries the message. axum and Lambda call `reject_local_uri` before `/files/list`, `/notes/read` and `/notes/save`, and map the error to 400. Tauri does not call it.

### Error body and status mapping
- axum: `ApiError::into_response` returns `Json({"error": message})`, and gains a `NotFound` → 404 mapping for notes.
- Lambda keeps its `error_response`.

Each server maps the `seqa_api` errors the same way:

| Shared error | Status |
|---|---|
| `SearchRequestError` (any variant) | 400 |
| `NotesError::InvalidPath`, `NotesError::InvalidData` | 400 |
| `NotesError::NotFound` | 404 |
| `NotesError::Store` | 500 |
| search failure | 500 |

The axum fallback returns the JSON 404 too, instead of HTML.

### Routing
- **axum**:
  - Remove `/files` and `list_dir`, and the axum-local `FileEntry`.
  - Register the gene-symbols route without the trailing slash.
  - Wrap the router with `tower_http::normalize_path::NormalizePath::trim_trailing_slash(router)` (tower-http feature `normalize-path`), so every route accepts a trailing slash. The wrapper must sit outside the router: a layer added with `Router::layer` runs after routing, too late to change which route matches. `app()` returns `NormalizePath<Router>`, and `main` serves it with `ServiceExt::<Request>::into_make_service`. Existing tests call `app().oneshot(...)`, which works on the wrapped service.
  - Register the JSON 404 handler as both `fallback` and `method_not_allowed_fallback`, so a wrong method on a known path (for example `GET /search`) returns the JSON 404 instead of axum's empty 405. Both servers use the message `Not found: {METHOD} {path}`.
  - *Alternative considered:* registering each route twice. Rejected as duplication.
- **Lambda**:
  - Add explicit arms for every route in the spec.
  - Replace `(POST, _)` with `(POST, ["search"])`.
  - Add a final `_` arm that returns the JSON 404.
  - Move cytobands to `["cytobands", genome, chromosome]`.
  - Lambda creates `StoreService::new()` per request today. Keep that.

### JSON bodies only
Both servers accept POST bodies only when the content type is `application/json` (parameters such as `; charset=utf-8` are allowed), and reject anything else with 400. axum's `Json` extractor already does this. Lambda replaces `RequestPayloadExt::payload` with a helper that checks the content type and then calls `serde_json::from_slice`. This stops any web page from writing notes with a cross-origin form post, which browsers send without a preflight.

### HEAD and OPTIONS
These are left as each framework handles them, and the spec allows it:
- axum answers `HEAD` on GET routes automatically, and Lambda returns the JSON 404.
- `OPTIONS` preflight is answered by axum's `CorsLayer`. For Lambda it is answered by API Gateway's CORS settings.

### Versions
- `seqa_api` 0.1.0: new crate.
- `seqa_core`: unchanged at 10.0.0. Every change made to it in the first implementation is reverted.
- `seqa_axum` 7.0.0 and `seqa_lambda` 5.0.0: breaking route and error-format changes.

These are workspace members with no root version, so no tag is needed. `seqa_api` 0.1.0 is published to crates.io before the seqa-mono change is applied.

## Risks / Trade-offs

- **Write exposure.** Any client CORS allows can overwrite `.../notes/...` objects in buckets the server's credentials can write to. → Mitigated by the notes-segment rule and the local-path rejection. Authentication is still out of scope, as it is for `/search`. This should be addressed before a public deployment.
- **Body size.** Large images or audio exceed the body limit (axum 2 MB, Lambda 6 MB). → Accepted for now. Raising the limit or using presigned URLs is future work.
- **Breaking Lambda clients.** → seqa-mono is the only client, and the paired change updates it in the same release.
- **Search validation is stricter.** Unknown `output_format` names and unrecognized extensions now return 400 on Lambda, which used to search anyway or fail with 500. → This is intended, and it matches axum's existing 400 for unsupported files.

## Migration Plan

1. Merge seqa-rs and publish `seqa_api` 0.1.0.
2. Deploy `seqa_lambda` 5.0.0 and `seqa_axum` 7.0.0.
3. Apply the paired seqa-mono change, which adds `seqa_api` 0.1.0 to Tauri and updates the UI services.

Rollback: redeploy the previous Lambda/axum versions together with the previous UI build.

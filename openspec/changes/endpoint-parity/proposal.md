# Proposal

Paired with seqa-mono change `endpoint-parity` (`seqa-mono/openspec/changes/endpoint-parity/`), which updates the Tauri commands and the UI services. This change covers a new `seqa_api` crate, `seqa_axum` and `seqa_lambda`. `seqa_core` is not changed.

## Why

The UI is one app with three backends: Tauri, axum and Lambda. Today the backends expose different endpoints:
- Lambda sends any POST to search, serves cytobands at a different path, and has no file browsing.
- Neither server can read or save notes.
- Each server has its own request and response types and its own error format.

The UI's HTTP service needs to call identical endpoints on axum and Lambda. A new `seqa_api` crate holds the request handling that all three backends share. That keeps the backends alike, and leaves `seqa_core` as a search engine with no API or notes code.

## What Changes

The endpoint set for both servers (and, through the paired change, the Tauri commands):

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/search` | Feature search |
| `POST` | `/files/list` | List one directory level |
| `GET` | `/files/buckets/{backend}` | List buckets/containers |
| `GET` | `/files/formats` | Supported file formats |
| `GET` | `/genes/symbols/{genome}/{source}` (trailing `/` optional) | Gene symbols |
| `GET` | `/cytobands/{genome}/{chromosome}` (trailing `/` optional) | Cytobands |
| `POST` | `/notes/read` | Read a note as base64 |
| `POST` | `/notes/save` | Save a note from base64 |

- New workspace crate `seqa_api`, which depends on `seqa_core`:
  - Shared request/response types and their conversions: the search request and response, the list-directory request, and the note read/save request and response.
  - A `notes` module: the notes-directory path rule, base64 read/save.
  - The local-URI guard (`is_local_uri`, `reject_local_uri`).
- `seqa_core`: no changes.
- `seqa_axum`:
  - Add `/notes/read` and `/notes/save`.
  - Accept an optional trailing `/` on gene symbols and cytobands.
  - Search accepts `index_path` and `output_format`.
  - **BREAKING** Remove `POST /files` (recursive listing).
  - **BREAKING** Error bodies become `{"error": "<message>"}` JSON instead of plain text.
- `seqa_lambda`:
  - Add `/files/list`, `/files/buckets/{backend}`, `/files/formats`, `/notes/read` and `/notes/save`.
  - **BREAKING** Search is only at `POST /search`, and unknown paths return 404.
  - **BREAKING** Cytobands move from `/genes/cytobands/...` to `/cytobands/...`.
  - **BREAKING** The search request drops `genome`.
- Both servers reject local (`file:`) and scheme-less URIs on `/files/list`, `/notes/read` and `/notes/save`.
- Both servers accept JSON bodies only. Form-encoded bodies are rejected with 400.
- Versions: `seqa_api` 0.1.0 (new), `seqa_core` unchanged at 10.0.0, `seqa_axum` 6.0.1 → 7.0.0, `seqa_lambda` 4.0.1 → 5.0.0.

Out of scope:
- App state (`/state`). For the axum and Lambda builds, the UI will keep state in browser storage, in a later change.
- Authentication.
- Tauri and UI code. These are covered by the paired seqa-mono change.

## Capabilities

### New Capabilities
- `http-api`: the endpoint set, request/response shapes, error format and route matching shared by `seqa_axum` and `seqa_lambda`.
- `notes-storage`: the note path rules and the base64 read/save contract.

### Modified Capabilities
- `lambda-search` (moved here from seqa-mono): **removed**. Every requirement is now covered by `http-api`, for both servers.

## Impact

- New crate `seqa_api` (dependencies: `seqa_core`, `serde`, `thiserror`, `url`, `object_store`, `base64`). Published to crates.io so that seqa-mono's Tauri app can use it.
- `seqa_core`: unchanged.
- `seqa_axum/src/main.rs`, `seqa_axum/src/search/models.rs`: routes, handlers and error format.
- `seqa_lambda/src/http_handler.rs`, `seqa_lambda/README.md`: routing and handlers.
- Body size: payloads are limited by axum's default body limit (2 MB) and the Lambda payload limit (6 MB). Base64 adds about 33%.

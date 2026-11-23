# seqa_api

Request handling shared by the seqa backends: `seqa_axum`, `seqa_lambda` and the seqa Tauri app. Every backend uses the same request and response types, so one UI client works against any of them. `seqa_core` does the searching; this crate holds the API code around it.

## Contents

| Module | Purpose |
|--------|---------|
| `search_request` | `SearchRequest`, the `POST /search` body, and `to_search_options`, which validates it and builds `SearchOptions` |
| `search_response` | `SearchResponse`, the `POST /search` response, built from a `SearchResult` |
| `list_directory_request` | `ListDirectoryRequest`, the `POST /files/list` body |
| `notes` | `read_note`, `save_note` and `validate_note_path`, with the `POST /notes/read` and `POST /notes/save` request and response types |
| `local_uri` | `is_local_uri` and `reject_local_uri`, which the HTTP servers use to refuse `file:` URIs and scheme-less paths |

## Usage

```rust
use seqa_api::search_request::SearchRequest;
use seqa_api::search_response::SearchResponse;
use seqa_core::api::search::search_features;
use seqa_core::stores::StoreService;

let request: SearchRequest = serde_json::from_str(body)?;
let options = request.to_search_options()?;
let store = StoreService::new();
let result = search_features(&store, &options).await?;
let response = SearchResponse::from_result(result, &options);
```

## Notes

A note path must be a `file`, `s3`, `gs` or `az` URI with a directory named exactly `notes` before a non-empty file name, e.g. `s3://my-bucket/analysis/notes/today.txt`. For `az` URIs the container is not part of that check. Note contents travel as standard base64 with padding.

`read_note` and `save_note` accept `file` URIs so that the Tauri app can keep notes on local disk. The HTTP servers call `reject_local_uri` first, so over HTTP only `s3`, `gs` and `az` notes are reachable.

## Testing

```bash
cargo test -p seqa_api
```

Some tests read and write `s3://com.gmail.docarw/test_data/`, so they need the AWS credentials described in [seqa_core/README.md](../seqa_core/README.md#environment-variables).

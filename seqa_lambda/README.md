# seqa_lambda

An AWS Lambda function (HTTP, via API Gateway) that serves the same endpoints as `seqa_axum`: genomic searches, file browsing, notes and reference data.

## Routes

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/search` | Genomic range query |
| `POST` | `/files/list` | List one directory level |
| `GET`  | `/files/buckets/{backend}` | List buckets or containers; `backend` is `s3`, `gcs` or `azure` |
| `GET`  | `/files/formats` | Supported file formats |
| `GET`  | `/genes/symbols/{genome}/{source}` | Gene coordinates; `source` is `refseq` or `gencode` |
| `GET`  | `/cytobands/{genome}/{chromosome}` | Cytobands for a chromosome |
| `POST` | `/notes/read` | Read a note as base64 |
| `POST` | `/notes/save` | Save a note from base64 |

`genome` is one of `hg19`, `hg38`, `grch37` or `grch38`. A trailing `/` on a path is optional. Any other method and path returns 404.

POST bodies must be JSON with content type `application/json`. Any other body, including a form-encoded one, returns 400. The request and response types come from [`seqa_api`](../seqa_api/README.md), so they match `seqa_axum` and the Tauri app.

### Errors

Every error response has content type `application/json` and the body:

```json
{ "error": "No coordinates" }
```

### `POST /search`

Request:

```json
{
    "path": "s3://my-bucket/sample.bam",
    "coordinates": ["chr12:10000000-10010000", "chr12:20000000-20010000"],
    "with_header": false,
    "only_header": false
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `path` | yes | URI of the data file; the format and index path are inferred from its extension unless set below |
| `coordinates` | yes | A list of one or more regions, 1-based inclusive, e.g. `["chr1:1000-2000"]` |
| `with_header` | no | Return the file header in `header`. Defaults to `false` |
| `only_header` | no | Return the header and no records. Defaults to `false` |
| `index_path` | no | URI of the index file, if it isn't next to the data file |
| `output_format` | no | File format (`bam`, `vcf`, `bed`, `bedpe`, `bedgraph`, `gff`, `gtf`, `fasta`, `bigwig`, `bigbed`), if it can't be inferred from the extension |

Other fields are ignored. The response is 400 when `coordinates` is empty (`No coordinates`) or not a list, a coordinate does not parse, `output_format` names an unknown format, or the format cannot be determined. A search that fails after validation returns 500.

Response (200):

```json
{
    "coordinates": ["chr12:10000000-10010000", "chr12:20000000-20010000"],
    "header": [],
    "lines": [["...SAM record...", "..."], ["...SAM record..."]]
}
```

`lines` has one array per entry in `coordinates`. An array is empty when the coordinate matches nothing, names a chromosome the file doesn't contain, or is over the format's size limit. `header` is empty unless `with_header` or `only_header` is `true`.

### `POST /files/list`

Request:

```json
{ "uri": "s3://my-bucket/analysis/" }
```

Response (200): the entries directly under `uri`. Each `uri` can be passed back to `/files/list`.

```json
[
    { "uri": "s3://my-bucket/analysis/notes/", "name": "notes", "isDirectory": true, "size": 0, "lastModified": "" },
    { "uri": "s3://my-bucket/analysis/sample.bam", "name": "sample.bam", "isDirectory": false, "size": 1024, "lastModified": "2026-01-01T00:00:00+00:00" }
]
```

`file:` URIs and scheme-less paths return 400. A malformed body returns 400 and a store failure returns 500.

### `GET /files/buckets/{backend}`

Response (200): a JSON array of bucket or container names. Any backend other than `s3`, `gcs` or `azure` (including `local`) returns 400.

### `POST /notes/read`

Request:

```json
{ "path": "s3://my-bucket/analysis/notes/today.txt" }
```

Response (200): the object's contents as standard base64 with padding.

```json
{ "data": "SGVsbG8gd29ybGQ=" }
```

### `POST /notes/save`

Request:

```json
{ "path": "s3://my-bucket/analysis/notes/today.txt", "data": "SGVsbG8gd29ybGQ=" }
```

Response: 204 with an empty body. The object is created or overwritten.

### Note paths

A note path must be an `s3://`, `gs://` or `az://` URI with a directory named exactly `notes` before a non-empty file name, e.g. `s3://my-bucket/analysis/notes/today.txt`. `file:` URIs and scheme-less paths are rejected.

| Status | When |
|--------|------|
| 400 | Invalid path, local path, invalid base64, or malformed body |
| 404 | `/notes/read` of a path with no object |
| 500 | Any other store failure; the message names the path |

Payloads are limited to 6 MB by Lambda, and base64 adds about 33%.

Cloud credentials are read from the Lambda's environment — see [seqa_core/README.md](../seqa_core/README.md#environment-variables).

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)

## Building

To build the project for production, run `cargo lambda build --release`. Remove the `--release` flag to build for development.

Read more about building your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/build.html).

## Testing

You can run regular Rust unit tests with `cargo test`.

If you want to run integration tests locally, you can use the `cargo lambda watch` and `cargo lambda invoke` commands to do it.

First, run `cargo lambda watch` to start a local server. When you make changes to the code, the server will automatically restart.

Second, you'll need a way to pass the event data to the lambda function.

You can use the existent [event payloads](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) in the Rust Runtime repository if your lambda function is using one of the supported event types.

You can use those examples directly with the `--data-example` flag, where the value is the name of the file in the [lambda-events](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) repository without the `example_` prefix and the `.json` extension.

```bash
cargo lambda invoke --data-example apigw-request
```

For generic events, where you define the event data structure, you can create a JSON file with the data you want to test with. For example:

```json
{
    "command": "test"
}
```

Then, run `cargo lambda invoke --data-file ./data.json` to invoke the function with the data in `data.json`.

For HTTP events, you can also call the function directly with cURL or any other HTTP client. For example:

```bash
curl https://localhost:9000
```

Read more about running the local server in [the Cargo Lambda documentation for the `watch` command](https://www.cargo-lambda.info/commands/watch.html).
Read more about invoking the function in [the Cargo Lambda documentation for the `invoke` command](https://www.cargo-lambda.info/commands/invoke.html).

## Deploying

To deploy the project, run `cargo lambda deploy`. This will create an IAM role and a Lambda function in your AWS account.

Read more about deploying your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/deploy.html).

# seqa-rs

[![CI](https://github.com/docwarren/seqa-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/docwarren/seqa-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/docwarren/seqa-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/docwarren/seqa-rs)

A set of Rust tools for making genomic range requests against files stored locally, over HTTP, or in cloud storage (AWS S3, Azure Blob Storage, Google Cloud Storage).

Supports BAM, VCF, GFF/GTF, BED, BedGraph, BigWig, BigBed, and FASTA formats. Indexes (BAI, TBI, FAI) are fetched automatically from the same storage backend as the data file. A single search can cover several regions of one file.

## Crates

| Crate | Description | README |
|-------|-------------|--------|
| [`seqa_core`](seqa_core/) | Core library — genomic file parsing, binary index reading, cloud storage via `object_store` | [seqa_core/README.md](seqa_core/README.md) |
| [`seqa_api`](seqa_api/) | Request and response types, notes read/save and the local-URI guard shared by `seqa_axum`, `seqa_lambda` and the Tauri app | [seqa_api/README.md](seqa_api/README.md) |
| [`seqa`](seqa_cli/) | CLI tool — query any supported file from the command line | [seqa_cli/README.md](seqa_cli/README.md) |
| [`seqa_axum`](seqa_axum/) | REST API server — genomic search, file browsing, notes and reference data | |
| [`seqa_lambda`](seqa_lambda/) | AWS Lambda function — the same endpoints as `seqa_axum`, behind API Gateway | [seqa_lambda/README.md](seqa_lambda/README.md) |

## Storage Backends

| Scheme | Backend |
|--------|---------|
| `/path/to/file`, `file://` | Local filesystem |
| `http://`, `https://` | HTTP/HTTPS |
| `s3://` | AWS S3 |
| `az://` | Azure Blob Storage |
| `gs://` | Google Cloud Storage |

## Quick Start

```bash
# Build everything
cargo build --release

# Query a local VCF file
seqa search /path/to/sample.vcf.gz chr1:1000000-2000000

# Query a file on S3
seqa search s3://my-bucket/sample.bam chr12:10000000-10010000

# Run the API server (listens on http://127.0.0.1:8000)
cargo run -p seqa_axum
```

## API Server

The `seqa_axum` crate provides a REST API backed by [axum](https://github.com/tokio-rs/axum). Run it with:

```bash
# Development (debug build)
cargo run -p seqa_axum

# Release build
cargo run -p seqa_axum --release
```

The server binds to `127.0.0.1:8000` and exposes:

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/search` | Genomic range query — see [Search request](#search-request) |
| `POST` | `/files/list` | List one level of a cloud URI (folders and files) — JSON body `{"uri": "s3://bucket/prefix/"}`; local paths are rejected |
| `GET`  | `/files/buckets/{backend}` | List the buckets/containers the configured credentials can see — `backend` is `s3`, `gcs` or `azure` |
| `GET`  | `/files/formats` | Supported file formats, their extensions and index extensions |
| `GET`  | `/genes/symbols/{genome}/{source}` | List gene symbols for `hg19`/`hg38`/`grch37`/`grch38`; `source` is `refseq` or `gencode` |
| `GET`  | `/cytobands/{genome}/{chromosome}` | Cytobands for a chromosome |
| `POST` | `/notes/read` | Read a note as base64 — JSON body `{"path": "s3://bucket/analysis/notes/today.txt"}`; local paths are rejected |
| `POST` | `/notes/save` | Save a note from base64 — JSON body `{"path": "...", "data": "<base64>"}`; local paths are rejected |

`seqa_lambda` serves the same endpoints; [seqa_lambda/README.md](seqa_lambda/README.md) documents every request, response and status code. A trailing `/` on a path is optional, and any other method and path returns 404. POST bodies must be JSON with content type `application/json`; anything else returns 400. Every error response is JSON:

```json
{ "error": "No coordinates" }
```

### Search request

```json
{
    "path": "s3://my-bucket/sample.vcf.gz",
    "coordinates": ["chr1:1000000-2000000"],
    "with_header": false,
    "only_header": false
}
```

`with_header` and `only_header` are optional and default to `false`. `index_path` and `output_format` are optional and override the index path and format inferred from the file extension. The response is JSON:

```json
{
    "coordinates": ["chr1:1000000-2000000"],
    "header": [],
    "lines": [["chr1\t1000123\t...", "..."]]
}
```

`lines` has one array per entry in `coordinates`. An array is empty when the coordinate matches nothing, names a chromosome the file doesn't contain, or is over the format's size limit. `header` is empty unless `with_header` or `only_header` is `true`.

Parsed indexes and headers are cached in memory between `/search` requests.

CORS is preconfigured for `http://localhost:5173` (Vite dev server) and `http://localhost:6006`/`6007` (Storybook). Cloud-backed requests require the same credentials as the CLI — see `seqa_core/README.md`.

Run the axum test suite:

```bash
cargo test -p seqa_axum
```

## Development

```bash
# Run unit tests (a few read from S3 and Azure, so they need credentials)
cargo test -p seqa_core --lib

# Run cloud integration tests (requires credentials)
cargo test -p seqa_core --test bam
cargo test -p seqa_core --test tabix
cargo test -p seqa_core --test bigwig
cargo test -p seqa_core --test bigbed
cargo test -p seqa_core --test fasta

# Run every test in the workspace
cargo test
```

### Code coverage
Requires `llvm-cov` and `cargo-llvm-cov` installed.

```bash
# Code coverage
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```
then you can 
```bash
cargo llvm-cov --open
```

See individual crate READMEs for credential setup.

# seqa-rs

[![CI](https://github.com/docwarren/seqa-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/docwarren/seqa-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/docwarren/seqa-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/docwarren/seqa-rs)

(Early release. Needs more thorough testing on a wide variety of files, as I have a limited test set.)

A set of Rust tools for making genomic range requests against files stored locally, over HTTP, or in cloud storage (AWS S3, Azure Blob Storage, Google Cloud Storage).

Supports BAM, VCF, GFF/GTF, BED, BedGraph, BigWig, BigBed, and FASTA formats. Indexes (BAI, TBI) are fetched automatically from the same storage backend as the data file.

## Crates

| Crate | Description | README |
|-------|-------------|--------|
| [`seqa_core`](seqa_core/) | Core library — genomic file parsing, binary index reading, cloud storage via `object_store` | [seqa_core/README.md](seqa_core/README.md) |
| [`seqa`](seqa_cli/) | CLI tool — query any supported file from the command line | [seqa_cli/README.md](seqa_cli/README.md) |
| [`seqa_axum`](seqa_axum/) | REST API server — HTTP endpoints for genomic search and file browsing |

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
| `GET`  | `/` | Health check |
| `POST` | `/search` | Genomic range query — JSON body `{"path": "...", "coordinates": "chr:start-end"}` |
| `POST` | `/files` | List objects under a storage URI — JSON body is a bare string, e.g. `"s3://bucket/prefix/"` |
| `GET`  | `/genes/symbols/{genome}` | List gene symbols for `hg19`/`hg38` |
| `GET`  | `/genes/coordinates/{genome}/{gene}` | Look up coordinates for a gene symbol |

CORS is preconfigured for `http://localhost:5173` (Vite dev server). Cloud-backed requests require the same credentials as the CLI — see `seqa_core/README.md`.

Run the axum test suite:

```bash
cargo test -p seqa_axum
```

## Development

```bash
# Run unit tests (no credentials needed)
cargo test -p seqa_core --lib
cargo test -p seqa_core --test read

# Run cloud integration tests (requires credentials)
cargo test -p seqa_core --test bam
cargo test -p seqa_core --test tabix
cargo test -p seqa_core --test bigwig
cargo test -p seqa_core --test bigbed
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

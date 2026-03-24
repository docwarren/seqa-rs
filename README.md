# soma-rs

(Early release. Needs more thorough testing on a wide variety of files, as I have a limited test set.)

A set of Rust tools for making genomic range requests against files stored locally, over HTTP, or in cloud storage (AWS S3, Azure Blob Storage, Google Cloud Storage).

Supports BAM, VCF, GFF/GTF, BED, BedGraph, BigWig, BigBed, and FASTA formats. Indexes (BAI, TBI) are fetched automatically from the same storage backend as the data file.

## Crates

| Crate | Description | README |
|-------|-------------|--------|
| [`soma_core`](soma_core/) | Core library — genomic file parsing, binary index reading, cloud storage via `object_store` | [soma_core/README.md](soma_core/README.md) |
| [`seqa`](soma_cli/) | CLI tool — query any supported file from the command line | [soma_cli/README.md](soma_cli/README.md) |
| [`soma_rocket`](soma_rocket/) | REST API server — HTTP endpoints for genomic search and file browsing | [soma_rocket/README.md](soma_rocket/README.md) |

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

# Run the API server
cargo run -p soma_rocket
```

## Development

```bash
# Run unit tests (no credentials needed)
cargo test -p soma_core --lib
cargo test -p soma_core --test read

# Run cloud integration tests (requires credentials)
cargo test -p soma_core --test bam
cargo test -p soma_core --test tabix
cargo test -p soma_core --test bigwig
cargo test -p soma_core --test bigbed
```

See individual crate READMEs for credential setup.

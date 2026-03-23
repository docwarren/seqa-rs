# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Soma is a Rust workspace for querying genomic files (BAM, VCF, GFF, BED, BigWig, BigBed, FASTA) across cloud and local storage backends. It consists of three crates:

- **soma_core** — Core library: genomic file parsing, binary index reading, cloud storage (S3/Azure/GCS/HTTP/local via `object_store`), MongoDB/SQLite integrations. Feature gates: `mongo`, `sqlite`.
- **soma_cli** (binary: `seqa`) — CLI tool using `clap`. Commands: `search`, `filter`, `jam`.
- **soma_rocket** — Rocket REST API server with CORS, serving genomic search, gene lookups, and patient/user CRUD. Enables both `sqlite` and `mongo` features on soma_core.

## Build & Test Commands

```bash
cargo build                           # Build all crates
cargo build -p soma_core              # Build core only
cargo test -p soma_core               # Test core (no feature-gated modules)
cargo test -p soma_core --features sqlite,mongo  # Test with all features
cargo test -p soma_rocket             # Test rocket server
cargo run -p seqa -- search <file> <coordinates>  # Run CLI
cargo run -p soma_rocket              # Run API server
cargo test -p soma_core -- <test_name>  # Run a single test
```

## Architecture

### Coordinate System

All genomic formats preserve their native coordinates. Internal canonical format is **0-based half-open** `[begin, end)`. Use `feature.canonical_interval()` for interval math. Terminology: always `begin`/`end`, never `start`/`stop`.

| Format | System |
|--------|--------|
| BED, BAM, BigWig | 0-based half-open |
| VCF, GFF, GTF | 1-based closed |

### Storage Layer

`StoreService::from_uri()` auto-detects backend from URL scheme (`s3://`, `az://`, `gs://`, `file://`, `http(s)://`). All file access goes through the `object_store` crate.

### Search Pipeline

`soma_core::api` exposes per-format search functions (`bam_search`, `tabix_search`, `fasta_search`, `bigwig_search`, `bigbed_search`), each taking `SearchOptions` and returning `SearchResult`. Both the CLI and Rocket server dispatch to these.

### Index Parsing

Binary index files (BAI, tabix, BigWig/BigBed) are parsed at the byte level with endianness handling. Uses SAM/BAM binning scheme (`bin`, `bin_util`, `chunk`, `virtual_offset`). Key traits: `SamIndex`, `WriteIndex`, `OptimizeOffsets`.

## Environment Variables

Cloud storage and database access require environment variables:
- **S3**: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `S3_BUCKET`
- **Azure**: `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_STORAGE_CONTAINER`, `AZURE_STORAGE_ACCOUNT`
- **GCS**: `GOOGLE_STORAGE_ACCOUNT`, `GOOGLE_BUCKET`
- **MongoDB**: `MONGO_CONNECTION`, `MONGO_DB_NAME`, `MONGO_USERNAME`, `MONGO_PASSWORD`

## Rust Edition

All crates use **Rust edition 2024**.

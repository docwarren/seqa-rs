# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Soma is a Rust workspace for querying genomic files (BAM, VCF, GFF, BED, BigWig, BigBed, FASTA) across cloud and local storage backends. It consists of three crates:

- **seqa_core** — Core library: genomic file parsing, binary index reading, cloud storage (S3/Azure/GCS/HTTP/local via `object_store`), SQLite integrations. Feature gate: `sqlite`.
- **seqa_cli** (binary: `seqa`) — CLI tool using `clap`. Commands: `search`, `filter`, `jam`.
- **seqa_rocket** — Rocket REST API server with CORS, serving genomic search, gene lookups, and patient/user CRUD. Enables the `sqlite` feature on seqa_core.

## Build & Test Commands

```bash
cargo build                           # Build all crates
cargo build -p seqa_core              # Build core only
cargo test -p seqa_core               # Test core (no feature-gated modules)
cargo test -p seqa_core --features sqlite  # Test with all features
cargo test -p seqa_rocket             # Test rocket server
cargo run -p seqa -- search <file> <coordinates>  # Run CLI
cargo run -p seqa_rocket              # Run API server
cargo test -p seqa_core -- <test_name>  # Run a single test
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

`seqa_core::api` exposes per-format search functions (`bam_search`, `tabix_search`, `fasta_search`, `bigwig_search`, `bigbed_search`), each taking `SearchOptions` and returning `SearchResult`. Both the CLI and Rocket server dispatch to these.

### Index Parsing

Binary index files (BAI, tabix, BigWig/BigBed) are parsed at the byte level with endianness handling. Uses SAM/BAM binning scheme (`bin`, `bin_util`, `chunk`, `virtual_offset`). Key traits: `SamIndex`, `WriteIndex`, `OptimizeOffsets`.

## Environment Variables

Cloud storage and database access require environment variables:
- **S3**: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `S3_BUCKET`
- **Azure**: `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_STORAGE_CONTAINER`, `AZURE_STORAGE_ACCOUNT`
- **GCS**: `GOOGLE_STORAGE_ACCOUNT`, `GOOGLE_BUCKET`

## Rust Edition

All crates use **Rust edition 2024**.

## Debugging Output Policy

**Never add `println!` or `eprintln!` statements** unless explicitly asked. These pollute stdout/stderr and break the user experience of CLI tools and the API server. Use the `log` crate (`log::debug!`, `log::info!`, etc.) if observability is needed.

## Test Deletion Policy

**Never delete a failing test** to make CI green. Before removing any test, you must:

1. Identify every assertion and code path exercised by the test.
2. Find an existing *passing* test that covers each of those cases — not just "similar" functionality, but the same inputs, the same code path, and equivalent assertions.
3. If full coverage cannot be confirmed, mark the test `#[ignore]` with a comment explaining why (e.g., machine-specific path, missing credentials) rather than deleting it.
4. Prefer moving credential-dependent tests to the integration test files (`tests/bam.rs`, `tests/tabix.rs`, etc.) over removing them.

Silently dropping tests to unblock CI is not acceptable. It creates gaps that are hard to detect later.

# soma_core

Core Rust library for genomic file parsing, cloud storage, and database access.

## Modules

- `api/` — High-level search functions: `bam_search`, `tabix_search`, `fasta_search`, `bigwig_search`, `bigbed_search`. Each takes `SearchOptions` and returns `SearchResult`. Also contains `OutputFormat` and shared constants.
- `codecs/` — Compression: `deflate`, `gzip`, `zlib` decoders for reading compressed genomic data.
- `genome/` — Genome reference data and utilities.
- `indexes/` — Binary index parsers:
  - `bai/` — BAM index (.bai) parsing with chromosome index support.
  - `tabix/` — Tabix index (.tbi) for VCF/GFF/BED.
  - `fai/` — FASTA index (.fai).
  - `bigwig/` — BigWig/BigBed binary format: headers, chromosome trees, R-trees, zoom levels, wig section headers.
  - Shared: `bin`, `bin_util`, `chunk`, `chunk_util`, `virtual_offset` — SAM/BAM binning scheme and virtual file offsets.
  - Traits: `SamIndex`, `WriteIndex`, `OptimizeOffsets`.
- `models/` — Data models: `coordinates`, `vcf`, `gff`, `gtf`, `bed`, `bedgraph`, `cytoband`, `gene_coordinate`, `tabix_header`.
- `stores/` — Cloud-agnostic file access via `object_store` crate. `StoreService` supports local filesystem, S3, Azure Blob, GCS, and HTTP. `from_uri()` auto-detects backend from URL scheme.
- `services/` — `SearchService` trait for genomic queries. Feature-gated `genes` module for SQLite.
- `traits/` — Shared traits like `Feature`.
- `utils.rs` — Path utilities, format detection, coordinate parsing.

### Feature-Gated Modules

- `sqlite` — Enables `sqlite/` module (gene symbol lookup, cytoband queries via `rusqlite`).

## Key Patterns

- **`StoreService::from_uri()`** — Detects storage backend from URL scheme (`s3://`, `az://`, `gs://`, `file://`, `http(s)://`).
- **Byte-level binary parsing** — Index files (BAI, tabix, BigWig) are parsed by reading raw bytes with endianness handling.
- **`SearchOptions`** — Unified search parameter struct used across all file-type search functions.

## Testing

- `cargo test -p soma_core`
- Test data in `data/` and `mock_data/` directories.
- Index-specific test data in `src/indexes/test_data.rs`.

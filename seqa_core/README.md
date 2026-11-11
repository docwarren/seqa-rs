# seqa_core

Core library for querying genomic files across cloud and local storage. Handles binary index parsing, cloud storage access, and genomic format support.

## Usage

```rust
use seqa_core::api::search::search_features;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::stores::StoreService;

let options = SearchOptions::new(
    "s3://my-bucket/sample.bam",
    &vec!["chr1:100000-200000".to_string(), "chr2:5000-6000".to_string()],
)?
.set_include_header(false);

let store_service = StoreService::new();
let result = search_features(&store_service, &options).await?;
for i in 0..result.coordinates.len() {
    println!("{}", result.coordinates[i]);
    for line in &result.lines[i] {
        println!("{}", line);
    }
}
```

- `SearchOptions::new` infers the index path and file format from the extension, and parses each coordinate string as **1-based closed**.
- A search can cover several regions. `result.lines` has one entry per region, in the order the regions were given, and `result.coordinates[i]` is the region for `result.lines[i]`.
- A region that is invalid for the file (a chromosome the file doesn't contain, a range past the end of a FASTA contig, or over the size limits below) gets an empty entry; the rest of the search still runs.
- The file header is returned in `result.header`, not in `lines`. `include_header` doesn't change what the search returns; it tells the caller whether to show the header. With `header_only`, every entry in `lines` is empty.
- The parsed index and header are returned in the `SearchResult`. Pass them back in through `SearchOptions` (`bam_index`, `bam_header`, `tabix_index`, `tabix_header`, `fasta_index`, `bigwig_index`, `bigbed_index`) to skip downloading them again on later queries.

### Coordinate strings

| Format | Example | Meaning |
|--------|---------|---------|
| Range | `chr1:1000000-2000000` | `begin` to `end`, inclusive |
| Single position | `chr1:1000000` | That one base |
| Chromosome | `chr1` | The whole chromosome |

Commas are stripped (`chr1:1,000,000-2,000,000`). Chromosome names with or without the `chr` prefix are accepted. Whole-chromosome queries use the longest length of that chromosome across hg19/hg38/GRCh37/GRCh38.

### Query size limits

| Format | Maximum bases per region |
|--------|-------------------------|
| BAM | 200,001 |
| FASTA | 250,000,000 |

A larger region gets an empty entry in `lines`. Other formats have no limit.

## Supported File Formats

| Format | Extensions | Index | Coordinate System |
|--------|------------|-------|------------------|
| BAM | `.bam` | `.bai` | 0-based half-open |
| VCF | `.vcf.gz` | `.tbi` | 1-based closed |
| GFF | `.gff.gz`, `.gff3.gz` | `.tbi` | 1-based closed |
| GTF | `.gtf.gz` | `.tbi` | 1-based closed |
| BED | `.bed.gz` | `.tbi` | 0-based half-open |
| BedGraph | `.bedgraph.gz` | `.tbi` | 0-based half-open |
| BigWig | `.bigwig`, `.bw` | embedded | 0-based half-open |
| BigBed | `.bigbed`, `.bb` | embedded | 0-based half-open |
| FASTA | `.fa`, `.fasta` | `.fai` | 1-based closed |

Text formats must be bgzipped and tabix-indexed; plain `.vcf`, `.bed`, etc. are not supported. Extensions are matched case-insensitively. The index is expected next to the data file with the index extension appended (e.g. `sample.bam` → `sample.bam.bai`), or can be set with `SearchOptions::set_index_path`.

## Storage Backends

`StoreService` picks the backend from the URL scheme of each path and builds each client the first time it is used, then reuses it for the same bucket or host:

| Scheme | Backend |
|--------|---------|
| `s3://` | AWS S3 |
| `az://` | Azure Blob Storage |
| `gs://` | Google Cloud Storage |
| `http://`, `https://` | HTTP |
| `file://`, local path (`/`, `./`, `../`) | Local filesystem |

## Index Caching

`.bai` and `.tbi` indexes for remote files are cached in the current working directory under the index's file name (e.g. `./sample.bam.bai`) and reused on later searches. Local indexes are read in place. Set `SearchOptions::set_no_cache(true)` to always download and never write the cache, or remove a cached file with `indexes::index_cache::delete_local_index`.

## Environment Variables

### AWS S3
```bash
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=...
```

### Azure Blob Storage
```bash
export AZURE_STORAGE_ACCOUNT_NAME=...
export AZURE_STORAGE_ACCESS_KEY=...
```

### Google Cloud Storage
```bash
# Option 1 (preferred): JSON key content directly as an env var
export GOOGLE_SERVICE_ACCOUNT_KEY='{ "type": "service_account", ... }'

# Option 2 (fallback): path to a service account JSON file
export GOOGLE_SERVICE_ACCOUNT=/path/to/credentials.json
```

## Features

- `sqlite` — enables gene symbol and cytoband lookups via SQLite (`rusqlite`)

```toml
seqa_core = "8"
```

## Testing

```bash
# Unit tests (no cloud credentials needed)
cargo test -p seqa_core --lib

# Everything, including the integration tests in tests/ (requires credentials above)
cargo test -p seqa_core
```

Most integration tests in `tests/` run against real data in S3, Azure and GCS, so they need the credentials above.

## Coordinate System

All formats preserve their native coordinates. Each `Coordinate` carries its `CoordinateSystem`, and the canonical internal format is **0-based half-open** `[begin, end)`. Use `Coordinate::to_canonical()` for interval math, and `Coordinate::overlaps()` to compare coordinates in any system. A record's coordinate is available from `Feature::coordinate()`. Terminology: always `begin`/`end`, never `start`/`stop`.

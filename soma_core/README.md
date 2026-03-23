# soma_core

Core library for querying genomic files across cloud and local storage. Handles binary index parsing, cloud storage access, and genomic format support.

## Supported File Formats

| Format | Index | Coordinate System |
|--------|-------|------------------|
| BAM | .bai | 0-based half-open |
| VCF | .tbi | 1-based closed |
| GFF/GTF | .tbi | 1-based closed |
| BED/BedGraph | .tbi | 0-based half-open |
| BigWig | embedded | 0-based half-open |
| BigBed | embedded | 0-based half-open |
| FASTA | .fai | 1-based closed |

## Storage Backends

`StoreService::from_uri()` detects the backend from the URL scheme:

| Scheme | Backend |
|--------|---------|
| `s3://` | AWS S3 |
| `az://` | Azure Blob Storage |
| `gs://` | Google Cloud Storage |
| `http://`, `https://` | HTTP |
| `file://`, local path | Local filesystem |

## Environment Variables

Set credentials for whichever backends you need.

### AWS S3
```bash
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=...
```

### Azure Blob Storage
```bash
export AZURE_STORAGE_ACCOUNT=...
export AZURE_STORAGE_CONTAINER=...
export AZURE_STORAGE_ACCESS_KEY=...
```

### Google Cloud Storage
```bash
# Option 1 (preferred): JSON key content directly as an env var
export GOOGLE_SERVICE_ACCOUNT_KEY='{ "type": "service_account", ... }'
export GOOGLE_BUCKET=...

# Option 2 (fallback): path to a service account JSON file
export GOOGLE_SERVICE_ACCOUNT=/path/to/credentials.json
export GOOGLE_BUCKET=...
```

## Features

- `sqlite` — enables gene symbol and cytoband lookups via SQLite (`rusqlite`)

```toml
soma_core = { path = "../soma_core", features = ["sqlite"] }
```

## Testing

```bash
# Unit tests and local file tests (no cloud credentials needed)
cargo test -p soma_core

# All tests including cloud storage (requires credentials above)
cargo test -p soma_core -- --include-ignored
```

Cloud storage tests are tagged `#[ignore]` and will be skipped unless explicitly included.

## Coordinate System

All formats preserve their native coordinates. The canonical internal format is **0-based half-open** `[begin, end)`. Use `feature.canonical_interval()` for interval math. Terminology: always `begin`/`end`, never `start`/`stop`.

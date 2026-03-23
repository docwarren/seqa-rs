# soma_rocket

REST API server for querying genomic files across cloud and local storage. Built with [Rocket](https://rocket.rs).

## Dependencies

- Rust (edition 2024)
- [Rocket](https://rocket.rs) 0.5
- [soma_core](../soma_core) — genomic file parsing and cloud storage
- SQLite gene databases in `data/` (e.g. `hg38-genes.db`, `hg19-genes.db`)

## Environment Variables

Cloud storage credentials must be set before running the server. The server will use whichever providers are configured — you only need to set the ones relevant to your storage backends.

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

## Installation

```bash
cargo build -p soma_rocket
```

## Running

```bash
cargo run -p soma_rocket
```

The server starts on `http://localhost:8000` by default.

### Development (live reload)

```bash
cargo install cargo-watch
cargo watch -p soma_rocket -x run
```

## Testing

```bash
cargo test -p soma_rocket
```

The test suite uses Rocket's in-process test client — no server needs to be running. Tests that hit cloud storage (VCF search, S3 file listing) require AWS credentials to be set.

## API Endpoints

### `GET /`
Health check. Returns `Hello world`.

### `POST /search`
Search a genomic file by coordinates.

**Request body (JSON):**
```json
{
  "path": "s3://my-bucket/sample.vcf.gz",
  "coordinates": "chr1:100000-200000",
  "limit": 100
}
```

Supported file types: `.bam`, `.vcf.gz`, `.bed.gz`, `.bedgraph.gz`, `.gff.gz`, `.fasta`, `.fa`, `.bw`, `.bigwig`, `.bb`, `.bigbed`

**Response:** newline-delimited records as plain text.

### `POST /files`
List files in a cloud storage directory.

**Request body (JSON):** a quoted path string
```json
"s3://my-bucket/my-data/"
```

**Response (JSON):**
```json
[
  { "path": "my-data/sample.vcf.gz", "lastModified": "2024-01-01T00:00:00Z", "size": 123456 }
]
```

### `GET /genes/symbols/<genome>`
List all gene symbols for a genome build.

- `genome`: `hg38` or `hg19`

**Response (JSON):** array of gene symbol strings.

### `GET /genes/coordinates/<genome>/<gene>`
Get genomic coordinates for a gene.

- `genome`: `hg38` or `hg19`
- `gene`: gene symbol (e.g. `BRCA1`)

**Response (JSON):**
```json
{
  "symbol": "BRCA1",
  "chromosome": "chr17",
  "start": 43044294,
  "end": 43125364
}
```

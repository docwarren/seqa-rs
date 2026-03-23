# seqa — Genomic File Query Tool

Fast command-line tool for querying genomic files across local and cloud storage.

## Supported File Formats

- **BAM** — Binary Alignment Map (requires `.bai` index)
- **VCF** — Variant Call Format (requires `.tbi` index)
- **GFF/GTF** — Gene Feature Format (requires `.tbi` index)
- **BED/BedGraph** — Browser Extensible Data (requires `.tbi` index)
- **BigWig** — Binary wiggle format (self-indexed)
- **BigBed** — Binary BED format (self-indexed)
- **FASTA** — Reference sequences (requires `.fai` index)

## Installation

```bash
cargo build --release -p seqa
```

The binary will be at `target/release/seqa`. To install it globally:

```bash
# Copy to /usr/local/bin
sudo cp target/release/seqa /usr/local/bin/

# Or symlink to ~/.local/bin (no sudo)
mkdir -p ~/.local/bin
ln -s $(pwd)/target/release/seqa ~/.local/bin/seqa
```

## Usage

```bash
seqa search <file> <coordinates> [OPTIONS]
```

### Coordinate Formats

| Format | Example | Description |
|--------|---------|-------------|
| Range | `chr1:1000000-2000000` | Records overlapping the range |
| Single position | `chr1:1000000` | Records overlapping that position |
| Full chromosome | `chr1` | All records on the chromosome |

Commas are stripped automatically: `chr1:1,000,000-2,000,000` works fine.

### Options

| Flag | Description |
|------|-------------|
| `-r, --reference <GENOME>` | Reference genome build: `hg38` or `hg19` (used for full-chromosome lengths) |
| `-w` | Include file header in output |
| `-o` | Output header only |

### Examples

```bash
# Local file
seqa search /path/to/sample.bam chr12:10000000-10010000

# Full chromosome with reference
seqa search /path/to/sample.vcf.gz chr1 -r hg38

# With header
seqa search /path/to/sample.gff.gz chr5:1000-2000 -w

# S3
seqa search s3://my-bucket/sample.bam chr1:1000000-2000000

# Azure
seqa search az://my-container/variants.vcf.gz chr12 -r hg19

# Google Cloud Storage
seqa search gs://my-bucket/annotations.gff.gz chr5:12345-67890

# HTTPS
seqa search https://example.com/public/file.bam chr1:1000000-1500000

# Pipe output
seqa search file.bam chr1:1000000-2000000 | wc -l
seqa search file.vcf.gz chr12 -r hg38 > chr12_variants.vcf
```

## Cloud Storage Credentials

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

> Never commit credentials to version control.

## Output

Results are written to stdout in each format's native representation:

| Input | Output |
|-------|--------|
| BAM | SAM |
| VCF | VCF |
| GFF/GTF | GFF/GTF |
| BED/BedGraph | BED/BedGraph |
| FASTA | FASTA |
| BigWig | Wiggle |
| BigBed | BED |

## Index Files

Index files must be in the same location as the data file with the standard extension appended (e.g. `sample.bam` → `sample.bam.bai`). For remote files the index is downloaded and cached locally for the duration of the query, then deleted.

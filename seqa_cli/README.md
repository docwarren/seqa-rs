# seqa — Genomic File Query Tool

Fast command-line tool for querying genomic files across local and cloud storage.

## Supported File Formats

- **BAM** — Binary Alignment Map (requires `.bai` index)
- **VCF** — Variant Call Format (requires `.tbi` index)
- **GFF/GTF** — General Feature Format / Gene Transfer Format (requires `.tbi` index)
- **BED/BedGraph** — Browser Extensible Data (requires `.tbi` index)
- **BigWig** — Binary wiggle format (self-indexed)
- **BigBed** — Binary BED format (self-indexed)
- **FASTA** — Reference sequences (requires `.fai` index)

VCF, GFF, GTF, BED and BedGraph files must be bgzipped (`.vcf.gz`, `.gff.gz`/`.gff3.gz`, `.gtf.gz`, `.bed.gz`, `.bedgraph.gz`); plain-text versions cannot be range-queried.

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
seqa genes <coordinates|symbols|cytobands> [OPTIONS]
```

### Coordinate Formats

Coordinates are 1-based and inclusive.

| Format | Example | Description |
|--------|---------|-------------|
| Range | `chr1:1000000-2000000` | Records overlapping the range |
| Begin only | `chr1:1000000` | Records from that position to the end of the chromosome |
| Full chromosome | `chr1` | All records on the chromosome |

Commas are stripped automatically: `chr1:1,000,000-2,000,000` works fine. Chromosome names with or without the `chr` prefix are accepted.

BAM searches are limited to 200,001 bases per query, so whole-chromosome and open-ended BAM queries are rejected. FASTA queries are limited to 250,000,000 bases.

### Search Options

| Flag | Description |
|------|-------------|
| `-r, --reference <GENOME>` | Reference genome build: `hg38` or `hg19`. Validated, but not currently used to resolve chromosome lengths |
| `-w true` | Include the file header in the output |
| `-o true` | Output the header only |
| `--no-cache` | Don't read or write the local index cache |

### Gene Commands

Gene data comes from the bundled SQLite databases. `-r, --reference` accepts `hg38`, `hg19`, `grch37` or `grch38`, and defaults to `grch38`.

```bash
# Coordinates (gene, chromosome, begin, end) for a gene symbol
seqa genes coordinates BRCA2

# Every gene symbol, one per line
seqa genes symbols -r hg19

# Cytobands for a chromosome
seqa genes cytobands chr1
```

### Examples

```bash
# Local file
seqa search /path/to/sample.bam chr12:10000000-10010000

# Full chromosome
seqa search /path/to/sample.vcf.gz chr1

# With header
seqa search /path/to/sample.gff.gz chr5:1000-2000 -w true

# S3
seqa search s3://my-bucket/sample.bam chr1:1000000-2000000

# Azure
seqa search az://my-container/variants.vcf.gz chr12

# Google Cloud Storage
seqa search gs://my-bucket/annotations.gff.gz chr5:12345-67890

# HTTPS
seqa search https://example.com/public/file.bam chr1:1000000-1500000

# Pipe output
seqa search file.bam chr1:1000000-2000000 | wc -l
seqa search file.vcf.gz chr12 -w true > chr12_variants.vcf
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

The bucket or container is taken from the URL, so it doesn't need to be set.

> Never commit credentials to version control.

## Output

Results are written to stdout in each format's native representation:

| Input | Output |
|-------|--------|
| BAM | SAM |
| VCF | VCF |
| GFF/GTF | GFF/GTF |
| BED/BedGraph | BED/BedGraph |
| FASTA | Sequence lines (no `>` header) |
| BigWig | Tab-separated `chromosome begin end value`, or zoom summaries for long regions |
| BigBed | BED |

## Index Files

Index files must be in the same location as the data file with the standard extension appended (e.g. `sample.bam` → `sample.bam.bai`). For remote BAM and tabix files the index is downloaded once and cached in the current directory under its file name (e.g. `./sample.bam.bai`), then reused by later searches. Pass `--no-cache` to always download it and never write the cache file.

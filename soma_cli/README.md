# seq - Genomic File Query Tool

seq is a fast, efficient command-line tool for querying genomic files across multiple storage backends. It supports local files, cloud storage (S3, Azure, GCS), and HTTP/HTTPS URLs.

## Prerequisites

- **Rust toolchain** (1.70 or later) - Install from https://rustup.rs/
- **Git** - For cloning the repository

## Supported File Formats

- **BAM** - Binary Alignment Map (requires .bai index)
- **VCF** - Variant Call Format (requires .tbi index)
- **GFF/GTF** - Gene Feature Format (requires .tbi index)
- **BED/BedGraph** - Browser Extensible Data (requires .tbi index)
- **BigWig** - Binary wiggle format (self-indexed)
- **BigBed** - Binary BED format (self-indexed)
- **FASTA** - Reference sequences (requires .fai index)

## Installation

### From Source

```bash
# Clone the repository and navigate to the workspace root
cd soma-rs

# Build the CLI tool
cargo build --release -p seq

# The binary will be available at target/release/seq
```

### Add to PATH (Optional)

To use `seq` from anywhere without specifying the full path, add it to your PATH:

**Option 1: Add the binary directory to PATH**

Add this line to your shell configuration file (`~/.bashrc`, `~/.zshrc`, or `~/.bash_profile`):

```bash
export PATH="$PATH:/path/to/soma-rs/target/release"
```

Then reload your shell configuration:
```bash
source ~/.bashrc  # or ~/.zshrc
```

**Option 2: Create a symlink in a directory already in PATH**

```bash
# Create a symlink in /usr/local/bin (may require sudo)
sudo ln -s /path/to/soma-rs/target/release/seq /usr/local/bin/seq

# Or in your local bin directory (no sudo required)
mkdir -p ~/.local/bin
ln -s /path/to/soma-rs/target/release/seq ~/.local/bin/seq
# Make sure ~/.local/bin is in your PATH
export PATH="$PATH:$HOME/.local/bin"
```

**Option 3: Copy the binary**

```bash
# Copy to /usr/local/bin (may require sudo)
sudo cp target/release/seq /usr/local/bin/

# Or to your local bin directory
mkdir -p ~/.local/bin
cp target/release/seq ~/.local/bin/
```

**Verify installation:**
```bash
which seq
seq --version
```

## Storage Backends

seqa automatically detects the storage backend from the file path:

- **Local files**: `/path/to/file.bam` or `file:///path/to/file.bam`
- **S3**: `s3://bucket/path/to/file.bam`
- **Azure Blob Storage**: `az://container/path/to/file.bam`
- **Google Cloud Storage**: `gs://bucket/path/to/file.bam`
- **HTTP/HTTPS**: `https://example.com/path/to/file.bam`

### Setting Up Cloud Storage Access

To access files on cloud storage platforms, you need to configure authentication credentials.

#### AWS S3

**Required Environment Variables:**
```bash
export AWS_ACCESS_KEY_ID=<your-access-key-id>
export AWS_SECRET_ACCESS_KEY=<your-secret-access-key>
export AWS_REGION=<region>  # e.g., us-east-1
```

**How to Create AWS Credentials:**

1. Go to the [AWS IAM Console](https://console.aws.amazon.com/iam/)
2. Navigate to "Users" → "Add users"
3. Enter username and select "Access key - Programmatic access"
4. Attach policy: "AmazonS3ReadOnlyAccess" (or "AmazonS3FullAccess" if needed)
5. Save the Access Key ID and Secret Access Key
6. Set the environment variables above

For detailed instructions, see [AWS IAM Documentation](https://docs.aws.amazon.com/IAM/latest/UserGuide/id_credentials_access-keys.html).

#### Azure Blob Storage

**Option 1: Using Service Principal (Recommended)**
```bash
export AZURE_TENANT_ID=<your-tenant-id>
export AZURE_CLIENT_ID=<your-client-id>
export AZURE_CLIENT_SECRET=<your-client-secret>
export AZURE_STORAGE_ACCOUNT=<storage-account-name>
export AZURE_STORAGE_CONTAINER=<container-name>
```

**How to Create Azure Service Principal:**

1. Go to the [Azure Portal](https://portal.azure.com/)
2. Navigate to "Azure Active Directory" → "App registrations" → "New registration"
3. Register your application
4. Go to "Certificates & secrets" → "New client secret"
5. Copy the client secret value immediately
6. Assign the app "Storage Blob Data Contributor" role on your storage account:
   - Storage account → "Access control (IAM)" → "Add role assignment"
7. Set the environment variables with your Tenant ID, Client ID (Application ID), and Client Secret

**Option 2: Using Access Keys**
```bash
export AZURE_STORAGE_ACCOUNT=<storage-account-name>
export AZURE_STORAGE_ACCESS_KEY=<access-key>
export AZURE_STORAGE_CONTAINER=<container-name>
```

For detailed instructions, see [Azure Storage Authentication Documentation](https://learn.microsoft.com/en-us/azure/storage/common/storage-auth).

#### Google Cloud Storage

**Required Environment Variables:**
```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json
```

**How to Create GCS Service Account:**

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Navigate to "IAM & Admin" → "Service Accounts"
3. Click "Create Service Account"
4. Enter name and grant role: "Storage Object Viewer" (read-only) or "Storage Object Admin" (read/write)
5. Click "Done"
6. Select your service account → "Keys" → "Add Key" → "Create new key"
7. Choose JSON format and download the key file
8. Set `GOOGLE_APPLICATION_CREDENTIALS` to the path of the downloaded JSON file

For detailed instructions, see [GCS Authentication Documentation](https://cloud.google.com/storage/docs/authentication).

#### Setting Environment Variables

You can set environment variables temporarily in your terminal:
```bash
export AWS_ACCESS_KEY_ID=your_key_here
export AWS_SECRET_ACCESS_KEY=your_secret_here
export AWS_REGION=us-east-1
```

Or permanently by adding them to your shell configuration file (`~/.bashrc`, `~/.zshrc`, etc.):
```bash
# Add to ~/.bashrc or ~/.zshrc
export AWS_ACCESS_KEY_ID=your_key_here
export AWS_SECRET_ACCESS_KEY=your_secret_here
export AWS_REGION=us-east-1
```

**Security Note:** Never commit credentials to version control. Consider using a `.env` file (not committed) or a secrets manager.

## Usage

### Basic Command Structure

```bash
seq search <file> <coordinates> [OPTIONS]
```

### Coordinate Formats

seq supports three coordinate formats:

1. **Full chromosome**: `chr12` or `12`
   - Returns all records on the chromosome
   - Uses genome-specific chromosome length if `-r` is specified
   - Otherwise uses the longest known chromosome length

2. **Single position**: `chr12:12000`
   - Returns records overlapping position 12000
   - Equivalent to `chr12:12000-12001`

3. **Range**: `chr12:12000-15000`
   - Returns records overlapping the range from 12000 to 15000

**Note:** Commas in coordinates are automatically stripped: `chr1:1,234,567-2,345,678` works fine.

### Options

- `-r, --reference <GENOME>` - Specify reference genome build (hg38 or hg19)
  - Used to determine chromosome lengths for full-chromosome queries
  - Example: `-r hg38` or `-r hg19`

- `-w` - Include header in output
  - Adds the file header (if available) to the output

- `-o` - Output header only
  - Returns only the file header without any records

### Examples

#### Local Files

```bash
# Query a BAM file with range
seq search /path/to/file.bam chr4:12345-13456

# Query entire chromosome with reference genome
seq search /path/to/file.bam chr12 -r hg38

# Query single position
seq search /path/to/file.vcf.gz chr1:12000

# Include header in output
seq search /path/to/file.gff.gz chr5:1000-2000 -w

# Get header only
seq search /path/to/file.bam chr1 -o
```

#### Cloud Storage

```bash
# S3
seq search s3://my-bucket/samples/sample1.bam chr1:1000000-2000000 -r hg38

# Azure
seq search az://my-container/data/variants.vcf.gz chr12 -r hg19

# Google Cloud Storage
seq search gs://my-bucket/annotations.gff.gz chr5:12345-67890

# HTTPS
seq search https://example.com/public/file.bam chr1:1000000-1500000
```

#### BigWig and BigBed (Self-Indexed)

```bash
# BigWig files don't require separate index files
seq search /path/to/file.bw chr4:12345-13456

# Same for BigBed
seq search /path/to/file.bb chr1:1000000-2000000
```

#### FASTA Files

```bash
# Query a reference sequence
seq search /path/to/genome.fa chr12:12000-15000

# Get entire chromosome sequence
seq search /path/to/genome.fa chr22 -r hg38
```

#### Using Numeric Chromosome Names

```bash
# These are equivalent to chr1, chr12, chrX
seq search file.bam 1:1000-2000
seq search file.bam 12
seq search file.bam X:5000000-6000000
```

## Output

seq writes results to standard output in the native format of each file type:
- BAM → SAM format
- VCF → VCF format
- GFF/GTF → GFF/GTF format
- BED/BedGraph → BED/BedGraph format
- FASTA → FASTA format
- BigWig → Wiggle format
- BigBed → BED format

Output can be piped to other tools or redirected to files:

```bash
# Pipe to grep
seq search file.bam chr1:1000-2000 | grep "MAPQ"

# Redirect to file
seq search file.vcf.gz chr12 -r hg38 > chr12_variants.vcf

# Count records
seq search file.bam chr1:1000000-2000000 | wc -l
```

## Coordinate Systems

seq preserves native coordinate systems:
- **0-based half-open [begin, end)**: BAM, BED, BigWig, BigBed
- **1-based closed [begin, end]**: VCF, GFF, GTF

When you specify `chr1:1000-2000`, the tool interprets this according to each format's convention.

## Index Files

Most formats require index files for efficient random access:

| Format | Index Extension | Auto-detected |
|--------|----------------|---------------|
| BAM | .bai | Yes |
| VCF | .tbi or .csi | Yes |
| GFF/GTF | .tbi | Yes |
| BED | .tbi | Yes |
| BigWig | (embedded) | N/A |
| BigBed | (embedded) | N/A |
| FASTA | .fai | Yes |

Index files must be in the same location as the data file with the standard extension.

## Error Handling

Common errors and solutions:

- **Index file not found**: Ensure the index file exists alongside the data file
- **Invalid coordinates**: Check chromosome name and coordinate format
- **Cloud access denied**: Verify environment variables are set correctly
- **Invalid reference genome**: Only `hg38` and `hg19` are supported for the `-r` option

## Performance Tips

- Use specific coordinate ranges when possible (faster than full chromosomes)
- For large queries, consider using cloud storage with high bandwidth
- Index files must be accessible (same storage backend as data files)

## License

See workspace LICENSE file.

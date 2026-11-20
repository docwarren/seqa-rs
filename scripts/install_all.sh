#!/bin/bash
set -euo pipefail
# Dependencies
# bgzip
# tabix
# wget
# awk
# sed
# zcat
# python3
# aws cli
# aws credentials in the ENV

# Run this script from a directory where you are happy to store lots of data.
# Every run starts clean and downloads everything again. Any failing command stops the
# run before anything is copied into seqa_core/data/ or uploaded to S3.
SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

CORE_DATA_DIR="$SCRIPT_PATH/../seqa_core/data/"
mkdir -p "${CORE_DATA_DIR}"

# Remove output from earlier runs
echo "=============================================================================================================="
echo "Removing output from earlier runs"
echo "=============================================================================================================="
rm -rf ./hg19 ./hg38 ./mane.bb

# Download latest gene information
echo "=============================================================================================================="
echo "Running get_transcript_data.sh"
echo "=============================================================================================================="
"${SCRIPT_PATH}"/get_transcript_data.sh

# Create gene coordinate CSV files
echo "=============================================================================================================="
echo "Running create_gene_coordinates.sh"
echo "=============================================================================================================="
"${SCRIPT_PATH}"/create_gene_coordinates.sh

# Download cytobands
echo "=============================================================================================================="
echo "Running download_cytobands.sh"
echo "=============================================================================================================="
"${SCRIPT_PATH}"/download_cytobands.sh

# Copy to data dir
CSVS=("./hg38/hg38.cytobands.csv" "./hg19/hg19.cytobands.csv" "./hg38/hg38.refseq.genes.csv" "./hg19/hg19.refseq.genes.csv" "./hg38/hg38.gencode.genes.csv" "./hg19/hg19.gencode.genes.csv")

echo "=============================================================================================================="
echo "Copying CSV files to data directory..."
echo "=============================================================================================================="
for csv in "${CSVS[@]}"; do
  echo "${csv} --> ${CORE_DATA_DIR}"
  cp "${csv}" "${CORE_DATA_DIR}"
done

# Upload gene data to s3
echo "=============================================================================================================="
echo "Uploading gene data to S3..."
echo "=============================================================================================================="
GENOMES=("hg38" "hg19")
for genome in "${GENOMES[@]}"; do
    for f in ./"${genome}"/*; do
        aws s3 cp "${f}" s3://com.soma23.data/"${genome}"/
    done
done

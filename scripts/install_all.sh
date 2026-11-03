#!/bin/bash
# Dependencies
# bgzip
# tabix
# wget
# sqlite3
# awk
# sed
# zcat
# aws cli
# aws credentials in the ENV
# Get the script path

# Run this script from a directory where you are happy to store lots of data
SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

CORE_DATA_DIR="$SCRIPT_PATH/../../seqa-rs/seqa_core/data/"
mkdir -p "${CORE_DATA_DIR}"

# Download latest gene information
echo "=============================================================================================================="
echo "Running get_transcript_data.sh"
echo "=============================================================================================================="
echo "${SCRIPT_PATH}"/get_transcript_data.sh

# Create gene_coordinates dbs
echo "=============================================================================================================="
echo "Running create_gene_coordinates_db.sh"
echo "=============================================================================================================="
echo "${SCRIPT_PATH}"/create_gene_coordinates_db.sh

# Download cytobands
echo "=============================================================================================================="
echo "Running download_cytobands.sh"
echo "=============================================================================================================="
echo "${SCRIPT_PATH}"/download_cytobands.sh

# Copy to data dir
DBS=("./hg38/hg38.cytobands.db" "./hg19/hg19.cytobands.db" "./hg38/hg38.refseq.gene.db" "./hg19/hg19.refseq.gene.db" "./hg38/hg38.gencode.gene.db" "./hg19/hg19.gencode.gene.db")

echo "=============================================================================================================="
echo "Copying databases to data directory..."
echo "=============================================================================================================="
for db in "${DBS[@]}"; do
  echo "${db} --> ${CORE_DATA_DIR}${db}"
  cp "${db}" "${CORE_DATA_DIR}"
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

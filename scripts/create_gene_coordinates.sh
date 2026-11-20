#!/bin/bash
set -euo pipefail

# Creates the gene coordinate CSV files that seqa_core bundles.
# Writes <genome>/<genome>.<source>.genes.csv with one chromosome,begin,end,symbol row per gene.
# Needs the <genome>/<genome>.gene.<source>.gtf.gz files from get_transcript_data.sh.
# Depends on
# python3
echo ""
echo "Creating gene coordinate CSV files"
echo "---------------------------------------------------------------------------------------------"
# --- Configuration ---
GENOMES=("hg19" "hg38")
SOURCES=("gencode" "refseq")
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)

for genome in "${GENOMES[@]}"; do
  for source in "${SOURCES[@]}"; do
    echo ""
    echo "Creating CSV for ${genome}/${source}"
    echo "----------------------------------------------------------------------------------------------"
    python3 "${SCRIPT_DIR}"/create_gene_csv.py -r "${genome}" -s "${source}"
  done
done

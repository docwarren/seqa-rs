#!/bin/bash
set -euo pipefail

# Downloads UCSC cytobands for hg38 and hg19 and writes <genome>/<genome>.cytobands.csv
# with one chromosome,begin,end,name,stain row per band. UCSC's placeholder rows for
# sequences without a cytogenetic map (alt, fix, unplaced and random contigs, chrM) have
# an empty name and are dropped.
# Depends on
# wget
# zcat
# awk
GENOMES=("hg38" "hg19")

for GENOME in "${GENOMES[@]}"; do
	echo ""
	echo "Downloading ${GENOME}/database/cytoBand.txt.gz"
	echo "--------------------------------------------------------------------------------------------"
	rm -f cytoBand.txt.gz

	wget "ftp://hgdownload.soe.ucsc.edu/goldenPath/${GENOME}/database/cytoBand.txt.gz"

	echo ""
	echo "Creating ./${GENOME}/${GENOME}.cytobands.csv"
	echo "--------------------------------------------------------------------------------------------"
	zcat cytoBand.txt.gz | awk -F'\t' -v OFS=, '$4 != "" { $1=$1; print }' > "./${GENOME}/${GENOME}.cytobands.csv"
	rm cytoBand.txt.gz
done

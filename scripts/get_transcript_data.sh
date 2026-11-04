#!/bin/bash
# Depends on:
#   extract_genes.py
#   lftp
#   wget
#   gunzip
#   bgzip
#   tabix

# Downloads and extracts gene data from NCBI and HGNC
# and chops it up into smaller files for indexing and looking up
# transcripts, exons, genes, pseudogenes etc
# BgZips and Tabix indexes all .gff files.
GENOMES=("hg19" "hg38")
features=("gene" "transcript" "exon" "CDS")

# MANE
#
echo ""
echo "Downloading MANE"
echo "-----------------------------------------------------------------------------------------------"
rm -f ./mane.bb
wget "https://hgdownload.soe.ucsc.edu/gbdb/hg38/mane/mane.bb"

for genome in "${GENOMES[@]}";
do
  mkdir -p "${genome}"
  gencode_uri=""
  gencode_local=""
  gencode_gtf="${genome}.gencode.gtf"
  refseq_uri=""
  refseq_local=""
  refseq_gtf="${genome}.refseq.gtf"

  if [[ "$genome" == "hg38" ]]; then
      gencode_uri="https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_50/gencode.v50.annotation.gtf.gz"
      gencode_local="./gencode.v50.annotation.gtf.gz"
      refseq_uri="https://ftp.ncbi.nlm.nih.gov/genomes/all/annotation_releases/9606/GCF_000001405.40-RS_2025_08/GCF_000001405.40_GRCh38.p14_genomic.gtf.gz"
      refseq_local="./GCF_000001405.40_GRCh38.p14_genomic.gtf.gz"
  fi
  if [[ "$genome" == "hg19" ]]; then
      gencode_uri="https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/latest_release/GRCh37_mapping/gencode.v50lift37.annotation.gtf.gz"
      gencode_local="./gencode.v50lift37.annotation.gtf.gz"
      refseq_uri="https://ftp.ncbi.nlm.nih.gov/genomes/all/annotation_releases/9606/GCF_000001405.25-RS_2024_09/GCF_000001405.25_GRCh37.p13_genomic.gtf.gz"
      refseq_local="./GCF_000001405.25_GRCh37.p13_genomic.gtf.gz"
  fi

  echo ""
  echo "Downloading GENCODE ${genome}"
  echo "-----------------------------------------------------------------------------------------------"

  if [[ ! -f ${gencode_local} ]]; then
      wget -nv "${gencode_uri}"
  fi

  echo ""
  echo "Downloading REFSEQ ${genome}"
  echo "-----------------------------------------------------------------------------------------------"
  if [[ ! -f ${refseq_local} ]]; then
      wget -nv "${refseq_uri}"
  fi

  echo ""
  echo "Renaming files"
  echo "-----------------------------------------------------------------------------------------------"

  mv "${gencode_local}" "${gencode_gtf}.gz"
  mv "${refseq_local}" "${refseq_gtf}.gz"

  gunzip "${gencode_gtf}.gz"
  gunzip "${refseq_gtf}.gz"

  echo ""
  echo "Replacing chromosome names with human readable names"
  echo "-----------------------------------------------------------------------------------------------"
  # Replace NC000 chromosome identifiers with normal chromosome names
  sed -i 's/NC_000001.11/chr1/g
          s/NC_000002.12/chr2/g
          s/NC_000003.12/chr3/g
          s/NC_000004.12/chr4/g
          s/NC_000005.10/chr5/g
          s/NC_000006.12/chr6/g
          s/NC_000007.14/chr7/g
          s/NC_000008.11/chr8/g
          s/NC_000009.12/chr9/g
          s/NC_000010.11/chr10/g
          s/NC_000011.10/chr11/g
          s/NC_000012.12/chr12/g
          s/NC_000013.11/chr13/g
          s/NC_000014.9/chr14/g
          s/NC_000015.10/chr15/g
          s/NC_000016.10/chr16/g
          s/NC_000017.11/chr17/g
          s/NC_000018.10/chr18/g
          s/NC_000019.10/chr19/g
          s/NC_000020.11/chr20/g
          s/NC_000021.9/chr21/g
          s/NC_000022.11/chr22/g
          s/NC_000023.11/chrX/g
          s/NC_000024.10/chrY/g
          s/NC_000001.10/chr1/g
          s/NC_000002.11/chr2/g
          s/NC_000003.11/chr3/g
          s/NC_000004.11/chr4/g
          s/NC_000005.9/chr5/g
          s/NC_000006.11/chr6/g
          s/NC_000007.13/chr7/g
          s/NC_000008.10/chr8/g
          s/NC_000009.11/chr9/g
          s/NC_000010.10/chr10/g
          s/NC_000011.9/chr11/g
          s/NC_000012.11/chr12/g
          s/NC_000013.10/chr13/g
          s/NC_000014.8/chr14/g
          s/NC_000015.9/chr15/g
          s/NC_000016.9/chr16/g
          s/NC_000017.10/chr17/g
          s/NC_000018.9/chr18/g
          s/NC_000019.9/chr19/g
          s/NC_000020.10/chr20/g
          s/NC_000021.8/chr21/g
          s/NC_000022.10/chr22/g
          s/NC_000023.10/chrX/g
          s/NC_000024.9/chrY/g' "$(pwd)/${refseq_gtf}"

  echo ""
  echo "Extracting genes, transcripts, exons, and cds into separate gtf files"
  echo "-----------------------------------------------------------------------------------------------"
  # Create gene, transcript, primary_transcript, exon, CDS, pseudogene GFF files
	for f in "${features[@]}";
	do
	  soma_gencode_unsorted="${genome}.${f}.gencode.unsorted.gtf"
	  soma_refseq_unsorted="${genome}.${f}.refseq.unsorted.gtf"
		soma_gencode_sorted="${genome}.${f}.gencode.gtf"
		soma_refseq_sorted="${genome}.${f}.refseq.gtf"

		echo ""
		echo "Extracting ${f}s..."
		echo "-----------------------------------------------------------------------------------------------"
		echo "Filtering..."
    awk -F '\t' '$3 ~ /'"${f}"'/' "${gencode_gtf}" | grep -v '^N' > "${soma_gencode_unsorted}"
		awk -F '\t' '$3 ~ /'"${f}"'/' "${refseq_gtf}" | grep -v '^N' > "${soma_refseq_unsorted}"

		# Sort by chromosome and start position
		echo "Sorting..."
		sort -t$'\t' -k1,1V -k4,4n "${soma_gencode_unsorted}" > "${soma_gencode_sorted}"
		sort -t$'\t' -k1,1V -k4,4n "${soma_refseq_unsorted}" > "${soma_refseq_sorted}"

		# BgZip and Tabix index GFF files
		echo "Compressing and indexing ..."
		bgzip -f "$(pwd)/${soma_gencode_sorted}"
		tabix -f -p gff "$(pwd)/${soma_gencode_sorted}.gz"

		bgzip -f "$(pwd)/${soma_refseq_sorted}"
		tabix -f -p gff "$(pwd)/${soma_refseq_sorted}.gz"
	done

	# Clean up unsorted files
  # Move all Grch37 && Grch38 files to a separate directory
	echo "Cleaning up..."
  echo "-----------------------------------------------------------------------------------------------"
  rm "${gencode_gtf}"
  rm "${refseq_gtf}"
  rm -- *.unsorted.*

	mv "${genome}".* "./${genome}"
	rm -f "$(pwd)/${genome}/*unsorted*"

done
mv ./mane.bb ./hg38/

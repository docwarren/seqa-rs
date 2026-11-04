GENOMES=("hg38" "hg19")

for ((i = 0; i < ${#GENOMES[@]}; i++)); do
	GENOME=${GENOMES[$i]}
	echo ""
	echo "Downloading ${GENOME}/database/cytoBand.txt.gz"
	echo "--------------------------------------------------------------------------------------------"
	if [[ -f cytoBand.txt.gz ]]; then
	  rm cytoBand.txt.gz
	fi

	wget ftp://hgdownload.soe.ucsc.edu/goldenPath/${GENOME}/database/cytoBand.txt.gz

	echo ""
	echo "Creating ${PREFIX}-cytobands.db"
	echo "--------------------------------------------------------------------------------------------"

	if [[ -f "${GENOME}".cytoband.tsv ]]; then
	  rm "${GENOME}".cytoband.tsv
	fi

	touch ${GENOME}.cytoband.tsv
	zcat cytoBand.txt.gz > ${GENOME}.cytoband.tsv
	sqlite3 "${GENOME}.cytobands.db" <<EOS
CREATE TABLE IF NOT EXISTS cytobands (
  chromosome TEXT,
  begin INTEGER,
  end INTEGER,
  name TEXT,
  stain TEXT
);

.mode tabs
.import ./${GENOME}.cytoband.tsv cytobands
.exit
EOS
  mv ${GENOME}.cytobands.db ./${GENOME}/${GENOME}.cytobands.db
  rm *.tsv
  rm *.txt.gz
done

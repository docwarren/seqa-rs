#!/bin/bash

# This script creates a SQLite database and imports gene coordinate data from CSV files into the database tables.
# Depends on
# python3
# sqlite3
echo ""
echo "Creating gene coordinates databases"
echo "---------------------------------------------------------------------------------------------"
# --- Configuration ---
GENOMES=("hg19" "hg38")
SOURCES=("gencode" "refseq")
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &> /dev/null && pwd)

echo ""
echo "Creating DBs"
echo "----------------------------------------------------------------------------------------------"

for genome in "${GENOMES[@]}"; do
  for source in "${SOURCES[@]}"; do
    csv_file="${genome}/${genome}.${source}.coordinates.csv"
    db_file="${genome}/${genome}.${source}.gene.db"
    # create coordinate csv files
    python3 "${SCRIPT_DIR}"/create_gene_csv.py -r "${genome}" -s "${source}"

    echo "Creating: ${db_file}"
    table="coordinates"

    # --- Import CSV into SQLite ---
    # Remove existing database file if it exists to start fresh
    if [ -f "${db_file}" ]; then
      rm "${db_file}"
      echo "Removed existing database: ${db_file}"
    fi

    echo "Importing '${csv_file}' into '${db_file}' table '${table}'..."

    # Use sqlite3's .import command within a "here document" (<<EOF)
    # or pipe commands directly
    sqlite3 "${db_file}" <<EOS

CREATE TABLE IF NOT EXISTS coordinates (
  chromosome TEXT,
  begin INTEGER,
  end INTEGER,
  symbol TEXT
);

.mode csv
.import "${csv_file}" "${table}"
EOS

    if [ $? -eq 0 ]; then
      echo "Import successful!"
    else
      echo "Error during CSV import."
      exit 1
    fi
  done
done

# Do this in an empty directory because there will be a lot of files downloaded and created

# Scripts for downloading gene data for GRCh38 and creating the relevant lookup tables in sqlite

## create_gene_coordinates.sh
This script creates a SQLite database and imports gene coordinate data from TSV files into the database tables.

**Dependent on** *get_gene_data.sh*
**Dependent on** sqlite

If you havent install sqlite3:
```
sudo apt update
sudo apt install sqlite3
```
### Usage
```bash
./create_gene_coordinates.sh
```

### Process
1. Remove any existing database file.
2. Create tables in the database based on the specified TSV files.

### Schema
Database name: genes.db

## create_gene_db.sh
This script creates a SQLite database and imports gene data from TSV files into the database tables.

**Dependent on** *create_gene_lookups.py*

### Usage
```bash
./create_gene_db.sh
```
### Process
1. Remove any existing database file.
2. Create tables in the database based on the specified TSV files.

### Schema
Database name: gene_coordinates.db

### Tables
#### Genes
| Column Name | Data Type |
|-------------|-----------|
| gene       | TEXT      |
| chr        | TEXT      |
| begin      | TEXT      |
| end        | TEXT      |

# Gene data scripts

Scripts for downloading gene annotations and cytobands for hg19 and hg38, and building the SQLite lookup databases that `seqa_core` embeds (the `sqlite` feature).

Run them from an empty directory. They download and create a lot of files, and write their output to `./hg19/` and `./hg38/` under the current directory.

## Dependencies

`wget`, `gunzip`, `zcat`, `bgzip`, `tabix`, `awk`, `sed`, `sort`, `sqlite3`, `python3`, and the AWS CLI with credentials in the environment (`install_all.sh` only).

```bash
sudo apt update
sudo apt install sqlite3 tabix wget
```

## install_all.sh

Runs the whole pipeline:

1. `get_transcript_data.sh`
2. `create_gene_coordinates_db.sh`
3. `download_cytobands.sh`
4. Copies the six databases into `seqa_core/data/`:
   - `hg38.cytobands.db`, `hg19.cytobands.db`
   - `hg38.refseq.gene.db`, `hg19.refseq.gene.db`
   - `hg38.gencode.gene.db`, `hg19.gencode.gene.db`
5. Uploads everything in `./hg38/` and `./hg19/` to `s3://com.soma23.data/<genome>/`.

```bash
/path/to/seqa-rs/scripts/install_all.sh
```

## get_transcript_data.sh

Downloads the GENCODE (v50) and RefSeq GTF annotations for hg19 and hg38, and the MANE BigBed (`mane.bb`) for hg38.

For each genome and source it:

1. Replaces RefSeq `NC_0000xx` accessions with `chr` names.
2. Splits the GTF into one file per feature type: `gene`, `transcript`, `exon`, `CDS`.
3. Sorts, bgzips and tabix-indexes each one, e.g. `hg38/hg38.gene.gencode.gtf.gz` and `hg38/hg38.gene.gencode.gtf.gz.tbi`.

`mane.bb` is moved to `./hg38/`.

## create_gene_coordinates_db.sh

Builds one gene database per genome and source: `<genome>/<genome>.<source>.gene.db`, for `hg19`/`hg38` and `gencode`/`refseq`. Needs the `*.gene.*.gtf.gz` files from `get_transcript_data.sh`.

For each combination it calls `create_gene_csv.py` to make `<genome>/<genome>.<source>.coordinates.csv`, then imports it into a fresh database:

| Table | Columns |
|-------|---------|
| `coordinates` | `chromosome TEXT`, `begin INTEGER`, `end INTEGER`, `symbol TEXT` |

## create_gene_csv.py

Reads `./<genome>/<genome>.gene.<source>.gtf.gz` and writes `./<genome>/<genome>.<source>.coordinates.csv` with one `chromosome,begin,end,symbol` row per gene. The symbol is taken from `gene_id` for RefSeq and `gene_name` for GENCODE.

```bash
python3 create_gene_csv.py -r hg38 -s refseq
```

## download_cytobands.sh

Downloads `cytoBand.txt.gz` for hg38 and hg19 from UCSC and imports it into `<genome>/<genome>.cytobands.db`. The `./hg38/` and `./hg19/` directories must already exist (`get_transcript_data.sh` creates them).

| Table | Columns |
|-------|---------|
| `cytobands` | `chromosome TEXT`, `begin INTEGER`, `end INTEGER`, `name TEXT`, `stain TEXT` |

## info.py

Not part of the pipeline. Prints statistics about the INFO field of a bgzipped VCF: the number of unique keys, which keys are categorical or numerical, and the number of unique values.

```bash
python3 info.py sample.vcf.gz
```

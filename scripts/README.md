# Gene data scripts

Scripts for downloading gene annotations and cytobands for hg19 and hg38, and building the CSV reference data that `seqa_core` bundles (`seqa_core::reference_data`).

Run them from an empty directory. They download and create a lot of files, and write their output to `./hg19/` and `./hg38/` under the current directory.

Every run starts clean: `install_all.sh` deletes `./hg19/`, `./hg38/` and `./mane.bb` first, and `get_transcript_data.sh` always downloads the source files again rather than reusing ones on disk. All scripts use `set -euo pipefail`, so the first failing command stops the run before anything is copied into `seqa_core/data/` or uploaded to S3.

## Dependencies

`wget`, `gunzip`, `zcat`, `bgzip`, `tabix`, `awk`, `sed`, `sort`, `python3`, and the AWS CLI with credentials in the environment (`install_all.sh` only).

```bash
sudo apt update
sudo apt install tabix wget
```

## Refreshing data and publishing

Refreshing the bundled data and publishing `seqa_core` are separate, manual steps:

1. From an empty working directory, run `/path/to/seqa-rs/scripts/install_all.sh`. It regenerates the six CSV files and copies them into `seqa_core/data/`.
2. Run `cargo test -p seqa_core`. The `reference_data_integrity` tests check row counts, chromosome names, coordinates, stains, cytoband tiling and key genes. Do not publish if they fail.
3. Review the diff of `seqa_core/data/` and commit it.
4. Run `cargo publish -p seqa_core`.

## install_all.sh

Runs the whole pipeline:

1. `get_transcript_data.sh`
2. `create_gene_coordinates.sh`
3. `download_cytobands.sh`
4. Copies the six CSV files into `seqa_core/data/`:
   - `hg38.cytobands.csv`, `hg19.cytobands.csv`
   - `hg38.refseq.genes.csv`, `hg19.refseq.genes.csv`
   - `hg38.gencode.genes.csv`, `hg19.gencode.genes.csv`
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

## create_gene_coordinates.sh

Builds one gene CSV per genome and source, `<genome>/<genome>.<source>.genes.csv`, for `hg19`/`hg38` and `gencode`/`refseq`, by calling `create_gene_csv.py` for each combination. Needs the `*.gene.*.gtf.gz` files from `get_transcript_data.sh`.

The files are headerless, with one `chromosome,begin,end,symbol` row per gene.

## create_gene_csv.py

Reads `./<genome>/<genome>.gene.<source>.gtf.gz` and writes `./<genome>/<genome>.<source>.genes.csv` with one `chromosome,begin,end,symbol` row per gene. The symbol is taken from `gene_id` for RefSeq and `gene_name` for GENCODE.

```bash
python3 create_gene_csv.py -r hg38 -s refseq
```

## download_cytobands.sh

Downloads `cytoBand.txt.gz` for hg38 and hg19 from UCSC and converts it to `<genome>/<genome>.cytobands.csv`: headerless, with one `chromosome,begin,end,name,stain` row per band. The `./hg38/` and `./hg19/` directories must already exist (`get_transcript_data.sh` creates them).

Rows with an empty band name are dropped. UCSC uses them as whole-sequence placeholders for sequences with no cytogenetic map: hg38's alt, fix, unplaced and random contigs, and `chrM`.

## tests/halt_on_failure.sh

Checks that `install_all.sh` removes earlier output and stops on a failed download without touching `seqa_core/data/` or calling `aws`. `wget` and `aws` are replaced by stubs, so it downloads and uploads nothing.

```bash
scripts/tests/halt_on_failure.sh
```

## info.py

Not part of the pipeline. Prints statistics about the INFO field of a bgzipped VCF: the number of unique keys, which keys are categorical or numerical, and the number of unique values.

```bash
python3 info.py sample.vcf.gz
```

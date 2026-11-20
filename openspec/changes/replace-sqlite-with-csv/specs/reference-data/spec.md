# Spec Delta

## Purpose

Provides gene coordinates and cytobands for the hg19 and hg38 genome builds from data bundled with `seqa_core`, so the CLI, HTTP server and Lambda can answer gene and cytoband lookups without any external database or file.

## ADDED Requirements

### Requirement: Reference data is bundled and needs no filesystem
Gene coordinate and cytoband lookups SHALL use only data compiled into the binary. They MUST NOT read from or write to the filesystem (including the system temp directory) and MUST NOT need a database engine.

#### Scenario: Available without opting in
- **GIVEN** a crate that depends on `seqa_core` with `default-features = false` and no features enabled
- **WHEN** it requests gene coordinates or cytobands
- **THEN** the lookups compile and return data

#### Scenario: Lookup with no writable temp directory
- **GIVEN** a process whose temp directory is not writable
- **WHEN** gene coordinates for `grch38`/`refseq` are requested
- **THEN** the lookup succeeds and returns the gene list

### Requirement: Genome build names are normalised
The genome build SHALL be matched case-insensitively. `hg19`, `grch37` and `ch37` SHALL select hg19. Every other value, including `hg38` and `grch38`, SHALL select hg38.

#### Scenario: GRCh37 alias
- **WHEN** gene coordinates are requested for genome `GRCh37` and source `refseq`
- **THEN** the hg19 RefSeq gene list is returned

#### Scenario: Unrecognised genome falls back to hg38
- **WHEN** cytobands are requested for genome `mm10` and chromosome `chr1`
- **THEN** the hg38 cytobands for `chr1` are returned

### Requirement: Gene coordinates by genome build and source
Gene coordinate lookup SHALL return every gene in the selected dataset, each with `chromosome`, `begin`, `end` and `symbol`, in the order the rows appear in the bundled data. Sources `refseq` and `gencode` SHALL select that source for the selected build. Any other source SHALL return the RefSeq list for the selected build.

#### Scenario: Key genes in every dataset
- **WHEN** gene coordinates are requested for each of `hg38`/`refseq`, `hg19`/`refseq`, `hg38`/`gencode` and `hg19`/`gencode`
- **THEN** each list contains `BRCA1` on `chr17`, `BRCA2` on `chr13` and `TP53` on `chr17`

#### Scenario: Unknown source
- **WHEN** gene coordinates are requested for `hg19`/`ensembl`
- **THEN** the hg19 RefSeq gene list is returned

### Requirement: Bundled gene data passes integrity checks
Every bundled gene dataset SHALL have at least 40,000 rows. In every row, `chromosome` SHALL be one of `chr1`–`chr22`, `chrX`, `chrY` or `chrM`, `symbol` SHALL be non-empty, `begin` SHALL be at least 1, and `begin` SHALL be no greater than `end`.

#### Scenario: All gene datasets pass
- **WHEN** each of the four gene datasets is loaded
- **THEN** it has at least 40,000 rows and every row meets the chromosome, symbol and coordinate rules

#### Scenario: Truncated regeneration is caught
- **GIVEN** a regenerated gene CSV that lost rows because a download was cut short
- **WHEN** the `seqa_core` test suite runs
- **THEN** the integrity tests fail

### Requirement: Cytobands by genome build and chromosome
Cytoband lookup SHALL return every cytoband in the selected build whose chromosome exactly equals the requested chromosome, each with `chromosome`, `begin`, `end`, `name` and `stain`, in bundled-data order. A chromosome with no cytobands SHALL return an empty list, not an error.

#### Scenario: hg38 chr1
- **WHEN** cytobands are requested for `grch38` and `chr1`
- **THEN** the list is non-empty and its first entry is `chr1`, `0`, `2300000`, `p36.33`, `gneg`

#### Scenario: hg19 chr1
- **WHEN** cytobands are requested for `hg19` and `chr1`
- **THEN** the list is non-empty and every entry has chromosome `chr1`

#### Scenario: Chromosome match is exact
- **WHEN** cytobands are requested for `grch38` and `1`
- **THEN** an empty list is returned

### Requirement: Bundled cytoband data passes integrity checks
Each build SHALL have bands for exactly `chr1`–`chr22`, `chrX` and `chrY`. Each band SHALL have `begin` less than `end` and a `stain` of `gneg`, `gpos25`, `gpos50`, `gpos75`, `gpos100`, `acen`, `gvar` or `stalk`. A chromosome's bands, in order, SHALL start at 0 and tile it with no gaps or overlaps.

#### Scenario: Both cytoband builds pass
- **WHEN** the hg38 and hg19 cytobands are loaded
- **THEN** each covers exactly the 24 chromosomes, and every band meets the coordinate, stain and tiling rules

#### Scenario: Gap between bands is caught
- **GIVEN** a regenerated cytoband CSV where one band's `begin` does not equal the previous band's `end`
- **WHEN** the `seqa_core` test suite runs
- **THEN** the integrity tests fail

### Requirement: Placeholder cytobands are excluded
The bundled cytobands SHALL NOT contain rows with an empty band name. UCSC uses these rows as whole-sequence placeholders for sequences that have no cytogenetic map.

#### Scenario: Alt contig has no cytobands
- **WHEN** cytobands are requested for `grch38` and `chr1_KI270762v1_alt`
- **THEN** an empty list is returned

#### Scenario: chrM has no cytobands
- **WHEN** cytobands are requested for `grch38` and `chrM`
- **THEN** an empty list is returned

#### Scenario: Every bundled cytoband has a name
- **WHEN** cytobands are requested for every chromosome in either build
- **THEN** every returned entry has a non-empty `name`

### Requirement: Data scripts download fresh and halt on failure
The data scripts SHALL delete earlier downloads and outputs before starting, so every run downloads from source. If any command in any script fails, the run SHALL stop with a non-zero exit status before copying files into `seqa_core/data/` or uploading anything.

#### Scenario: Earlier downloads are not reused
- **GIVEN** GTF files and outputs from an earlier run in the working directory
- **WHEN** `install_all.sh` runs
- **THEN** every source file is downloaded again

#### Scenario: Download failure halts the run
- **GIVEN** a download that fails
- **WHEN** `install_all.sh` runs
- **THEN** it exits non-zero, and `seqa_core/data/` and S3 are left unchanged

### Requirement: Each dataset is loaded at most once per process
A dataset SHALL be parsed into memory the first time it is requested and reused for every later request in the same process. Datasets that are never requested SHALL NOT be parsed.

#### Scenario: Repeated lookups reuse loaded data
- **WHEN** gene coordinates for `grch38`/`refseq` are requested twice in one process
- **THEN** both calls return the same data and the bundled data is parsed only once

### Requirement: Front ends keep their lookup interfaces
The CLI `genes coordinates`, `genes symbols` and `cytobands` commands, the axum routes `GET /genes/symbols/{genome}/{source}/` and `GET /cytobands/{genome}/{chromosome}`, and the Lambda routes `GET /genes/symbols/{genome}/{source}` and `GET /genes/cytobands/{genome}/{chromosome}` SHALL keep their paths, arguments and output formats.

#### Scenario: Lambda gene symbols
- **WHEN** the Lambda receives `GET /genes/symbols/grch38/refseq`
- **THEN** it responds 200 with a JSON array containing an object with `symbol` `BRCA2`

#### Scenario: axum gene symbols
- **WHEN** the axum server receives `GET /genes/symbols/grch38/refseq/`
- **THEN** it responds 200 with a JSON array containing an object with `symbol` `BRCA2`

#### Scenario: axum cytobands
- **WHEN** the axum server receives `GET /cytobands/grch38/chr1`
- **THEN** it responds 200 with a non-empty JSON array

#### Scenario: CLI cytobands
- **WHEN** `seqa cytobands chr1` is run
- **THEN** it prints one tab-separated `chromosome begin end name stain` line per hg38 `chr1` band

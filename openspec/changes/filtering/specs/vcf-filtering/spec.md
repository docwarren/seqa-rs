# Spec Delta

## Purpose

Lets callers narrow VCF search results to the records that match a JSON filter, built from INFO, sample, QUAL, FILTER, gene and coordinate conditions combined with AND/OR. The same rules apply across seqa_core, the CLI, the axum server and the lambda.

## ADDED Requirements

### Requirement: Filter document structure
A filter SHALL be a JSON object that is either a leaf filter with a `type` property or a combinator: `{"AND": [...]}` or `{"OR": [...]}` holding a non-empty array of filters. Combinators SHALL nest to any depth. A document that matches none of these shapes SHALL be rejected with a `SearchError`.

#### Scenario: AND of leaves
- **WHEN** the filter is `{"AND": [{"type":"QUAL","operator":"gt","value":30}, {"type":"FILTERS","operator":"contains","value":"PASS"}]}`
- **THEN** only records with QUAL above 30 and a PASS filter are returned

#### Scenario: Nested OR inside AND
- **WHEN** the filter is `{"AND": [{"type":"QUAL","operator":"gt","value":30}, {"OR": [{"type":"INFO","key":"DP","operator":"gte","value":10}, {"type":"INFO","key":"DB","operator":"exists"}]}]}`
- **THEN** a record is returned when its QUAL is above 30 and it has either DP ≥ 10 or the DB flag

#### Scenario: Empty combinator
- **WHEN** the filter is `{"AND": []}` or `{"OR": []}`
- **THEN** the search fails with a `SearchError`

#### Scenario: NOT is not supported
- **WHEN** the filter is `{"NOT": {"type":"QUAL","operator":"gt","value":30}}`
- **THEN** the search fails with a `SearchError`

### Requirement: Leaf filter types and properties
A leaf filter SHALL have a `type` of `INFO`, `SAMPLE`, `QUAL`, `FILTERS`, `GENES` or `COORDINATES` (uppercase), plus exactly the properties that type needs. A leaf with an unknown type, an unknown operator, a missing required property, or a property that type does not use SHALL be rejected with a `SearchError`.

#### Scenario: Properties per type
- **WHEN** a leaf filter is given
- **THEN** it is accepted only in one of these shapes:
  - `INFO`: `key`, `operator`, and `value` (except for `exists` / `!exists`)
  - `SAMPLE`: `sampleId`, `key`, `operator`, and `value` (except for presence and genotype operators)
  - `QUAL`: `operator`, `value`
  - `FILTERS`: `operator`, `value`
  - `GENES`: `values` only
  - `COORDINATES`: `operator`, `values`

#### Scenario: Lowercase type
- **WHEN** a leaf has `"type": "coordinates"`
- **THEN** the search fails with a `SearchError`

#### Scenario: Missing key
- **WHEN** a leaf is `{"type":"INFO","operator":"gt","value":10}`
- **THEN** the search fails with a `SearchError`

#### Scenario: Unused property
- **WHEN** a leaf is `{"type":"GENES","operator":"overlaps","values":["BRCA1"]}`
- **THEN** the search fails with a `SearchError`

### Requirement: Comparison kind follows the JSON value type
For `eq` and `neq`, the system SHALL compare numerically when `value` is a JSON number and as text when `value` is a JSON string. The numeric-only operators `lt`, `gt`, `lte` and `gte` SHALL require a JSON number. The text-only operators `contains` and `!contains` SHALL require a JSON string.

#### Scenario: Numeric eq
- **WHEN** the filter is `{"type":"INFO","key":"DP","operator":"eq","value":10}` and a record has `DP=10.0`
- **THEN** the record matches

#### Scenario: Text eq
- **WHEN** the filter is `{"type":"INFO","key":"DP","operator":"eq","value":"10"}` and a record has `DP=10.0`
- **THEN** the record does not match

#### Scenario: Numeric operator with a string value
- **WHEN** the filter is `{"type":"INFO","key":"DP","operator":"gt","value":"10"}`
- **THEN** the search fails with a `SearchError`

#### Scenario: contains with a number value
- **WHEN** the filter is `{"type":"INFO","key":"GENE","operator":"contains","value":5}`
- **THEN** the search fails with a `SearchError`

### Requirement: Numeric comparisons
Numeric operators (`eq`, `neq`, `lt`, `gt`, `lte`, `gte`) SHALL parse the record's value as a 32-bit float and compare it with the filter value.

#### Scenario: Greater than
- **WHEN** the filter is `{"type":"INFO","key":"AF","operator":"gt","value":0.5}`
- **THEN** a record with `AF=0.75` matches and a record with `AF=0.25` does not

#### Scenario: Less than or equal
- **WHEN** the filter is `{"type":"INFO","key":"DP","operator":"lte","value":20}`
- **THEN** a record with `DP=20` matches and a record with `DP=21` does not

### Requirement: VCF value of the wrong type is an error
When a numeric operator meets a record value that is present, is not `.`, and does not parse as a number, the system SHALL fail the search with a `SearchError` that names the key and the value. Text operators SHALL never fail for type reasons.

#### Scenario: Non-numeric INFO value
- **WHEN** the filter is `{"type":"INFO","key":"DP","operator":"gt","value":10}` and a record in the region has `DP=high`
- **THEN** the search fails with a `SearchError`

#### Scenario: Text comparison on a numeric field
- **WHEN** the filter is `{"type":"INFO","key":"DP","operator":"contains","value":"1"}` and a record has `DP=12`
- **THEN** the record matches and no error is raised

### Requirement: Text comparisons are case-insensitive
Text operators SHALL compare without regard to letter case. `eq` / `neq` SHALL compare the whole value. `contains` / `!contains` SHALL test whether the filter value appears as a substring.

#### Scenario: Case-insensitive eq
- **WHEN** the filter is `{"type":"INFO","key":"SVTYPE","operator":"eq","value":"del"}` and a record has `SVTYPE=DEL`
- **THEN** the record matches

#### Scenario: Case-insensitive contains
- **WHEN** the filter is `{"type":"INFO","key":"ANN","operator":"contains","value":"missense"}` and a record has `ANN=A|Missense_variant|MODERATE`
- **THEN** the record matches

#### Scenario: Negated contains
- **WHEN** the filter is `{"type":"INFO","key":"ANN","operator":"!contains","value":"intron"}` and a record has `ANN=A|missense_variant`
- **THEN** the record matches

### Requirement: Missing and `.` values never match
When the key is absent from the record, or its value is `.` or empty, every value operator (`eq`, `neq`, `lt`, `gt`, `lte`, `gte`, `contains`, `!contains`) SHALL evaluate to false, including the negated ones. Only `!exists` matches a missing value.

#### Scenario: Absent key with neq
- **WHEN** the filter is `{"type":"INFO","key":"DP","operator":"neq","value":10}` and a record has no DP field
- **THEN** the record does not match

#### Scenario: Dot value with !contains
- **WHEN** the filter is `{"type":"INFO","key":"GENE","operator":"!contains","value":"BRCA"}` and a record has `GENE=.`
- **THEN** the record does not match

### Requirement: Presence operators
For INFO and SAMPLE filters, `exists` SHALL match when the key is present and its value is not `.`, and `!exists` SHALL match otherwise. An INFO flag (a key with no `=value`) SHALL count as present. A `value` property is not allowed with these operators.

#### Scenario: INFO flag present
- **WHEN** the filter is `{"type":"INFO","key":"DB","operator":"exists"}` and a record's INFO is `DP=10;DB`
- **THEN** the record matches

#### Scenario: INFO flag absent
- **WHEN** the filter is `{"type":"INFO","key":"DB","operator":"!exists"}` and a record's INFO is `DP=10`
- **THEN** the record matches

#### Scenario: Sample value missing
- **WHEN** the filter is `{"type":"SAMPLE","sampleId":"NA12878","key":"AD","operator":"exists"}` and NA12878's AD value is `.`
- **THEN** the record does not match

### Requirement: Multi-value fields match like bcftools
When a record value holds several comma-separated values (e.g. `AF=0.1,0.6`), a value operator SHALL match if it matches any of the values. Individual `.` values SHALL be skipped. If every value is `.`, the field is treated as missing.

#### Scenario: Any value satisfies gt
- **WHEN** the filter is `{"type":"INFO","key":"AF","operator":"gt","value":0.5}` and a record has `AF=0.1,0.6`
- **THEN** the record matches

#### Scenario: neq on multiple values
- **WHEN** the filter is `{"type":"INFO","key":"AF","operator":"neq","value":0.1}` and a record has `AF=0.1,0.2`
- **THEN** the record matches, because 0.2 is not 0.1

#### Scenario: Missing element skipped
- **WHEN** the filter is `{"type":"INFO","key":"AF","operator":"lt","value":0.5}` and a record has `AF=.,0.2`
- **THEN** the record matches

### Requirement: SAMPLE filters select a sample by name
A SAMPLE filter SHALL compare the FORMAT field `key` for the sample whose column header in the VCF `#CHROM` line equals `sampleId`. Non-GT keys SHALL use the same operators and rules as INFO filters. An unknown `sampleId` SHALL fail the search with a `SearchError` before any records are read.

#### Scenario: Sample field comparison
- **WHEN** the filter is `{"type":"SAMPLE","sampleId":"NA12878","key":"DP","operator":"gte","value":15}` and NA12878's DP in a record is 20
- **THEN** the record matches

#### Scenario: Unknown sample
- **WHEN** the filter names `"sampleId":"NOPE"` and the VCF header has no such sample column
- **THEN** the search fails with a `SearchError`

### Requirement: Genotype operators on GT
The operators `na`, `ref`, `hom`, `het`, `father` and `mother` SHALL be accepted only on SAMPLE filters with `key` `GT` and SHALL take no `value`. A genotype is one or two alleles (a number or `.`) separated by `/` or `|`. A GT value that does not have this form SHALL fail the search with a `SearchError`.

#### Scenario: Genotype operator on another key
- **WHEN** the filter is `{"type":"SAMPLE","sampleId":"NA12878","key":"DP","operator":"het"}`
- **THEN** the search fails with a `SearchError`

#### Scenario: Malformed genotype
- **WHEN** a genotype filter meets a GT value of `0/1/2` or `A/B`
- **THEN** the search fails with a `SearchError`

### Requirement: Genotype class definitions
`na` SHALL match when every allele is `.`. `ref` SHALL match when every allele is `0`. `hom` SHALL match when every allele is the same number other than 0, including haploid calls. `het` SHALL match two called alleles with different numbers. `father` SHALL match phased (`|`) calls whose right allele is a number other than 0. `mother` SHALL match phased calls whose left allele is a number other than 0.

#### Scenario: na
- **WHEN** the GT values are `.`, `./.`, `.|.`, `./0`
- **THEN** `na` matches `.`, `./.` and `.|.`, but not `./0`

#### Scenario: ref
- **WHEN** the GT values are `0`, `0/0`, `0|0`, `./.`, `0/.`
- **THEN** `ref` matches `0`, `0/0` and `0|0` only

#### Scenario: hom
- **WHEN** the GT values are `1/1`, `2|2`, `1`, `0/0`, `1/2`, `./.`
- **THEN** `hom` matches `1/1`, `2|2` and `1` only

#### Scenario: het
- **WHEN** the GT values are `0/1`, `1/0`, `1|2`, `1/1`, `./1`, `1`
- **THEN** `het` matches `0/1`, `1/0` and `1|2` only

#### Scenario: father
- **WHEN** the GT values are `0|1`, `1|2`, `0|.`, `1|0`, `0/1`
- **THEN** `father` matches `0|1` and `1|2` only

#### Scenario: mother
- **WHEN** the GT values are `1|0`, `2|1`, `0|1`, `.|1`, `1/0`
- **THEN** `mother` matches `1|0` and `2|1` only

### Requirement: QUAL filter
A QUAL filter SHALL compare the record's QUAL column using the numeric operators, under the same rules as a numeric INFO field. A QUAL of `.` SHALL never match.

#### Scenario: QUAL above threshold
- **WHEN** the filter is `{"type":"QUAL","operator":"gt","value":30}`
- **THEN** a record with QUAL 50 matches, and records with QUAL 20 or `.` do not

#### Scenario: Text operator on QUAL
- **WHEN** the filter is `{"type":"QUAL","operator":"contains","value":"3"}`
- **THEN** the search fails with a `SearchError`

### Requirement: FILTERS filter
A FILTERS filter SHALL split the record's FILTER column on both `;` and `,`. `contains` SHALL match when some entry equals `value` (ignoring case). `!contains` SHALL match when no entry equals `value`. A FILTER column of `.` SHALL never match.

#### Scenario: contains PASS
- **WHEN** the filter is `{"type":"FILTERS","operator":"contains","value":"pass"}`
- **THEN** a record with FILTER `PASS` matches, and a record with FILTER `PASSED` does not

#### Scenario: Either delimiter
- **WHEN** the filter is `{"type":"FILTERS","operator":"contains","value":"LowQual"}`
- **THEN** records with FILTER `q10;LowQual` and with FILTER `q10,LowQual` both match

#### Scenario: !contains
- **WHEN** the filter is `{"type":"FILTERS","operator":"!contains","value":"FAIL"}`
- **THEN** a record with FILTER `PASS` matches, and a record with FILTER `q10;FAIL` does not

#### Scenario: Unset FILTER
- **WHEN** a record's FILTER column is `.`
- **THEN** neither `contains` nor `!contains` matches it

### Requirement: Record overlap for coordinate-based filters
For COORDINATES and GENES filters, a record with an `END` or `SVLEN` INFO value SHALL overlap a region when its begin breakpoint (CHROM:POS) or its end breakpoint lies in the region. The end breakpoint's chromosome is INFO `CHR2` if present, else CHROM. Its position is `END`, else POS + |SVLEN|. Other records SHALL use their normal span.

#### Scenario: SV begin breakpoint in region
- **WHEN** a record at `chr1:1000` has `SVTYPE=DEL;END=50000` and the region is `chr1:900-1100`
- **THEN** the record overlaps the region

#### Scenario: SV end breakpoint in region
- **WHEN** a record at `chr1:1000` has `END=50000` and the region is `chr1:49900-50100`
- **THEN** the record overlaps the region

#### Scenario: SV spanning a region with neither breakpoint in it
- **WHEN** a record at `chr1:1000` has `END=50000` and the region is `chr1:20000-30000`
- **THEN** the record does not overlap the region

#### Scenario: Translocation end on another chromosome
- **WHEN** a record at `chr1:1000` has `CHR2=chr5;END=7000` and the region is `chr5:6900-7100`
- **THEN** the record overlaps the region

#### Scenario: Negative SVLEN
- **WHEN** a record at `chr1:1000` has `SVLEN=-500` and no `END`, and the region is `chr1:1450-1550`
- **THEN** the record overlaps the region

#### Scenario: Small variant
- **WHEN** a record at `chr1:1000` is `REF=A ALT=G` and the region is `chr1:1000`
- **THEN** the record overlaps the region

### Requirement: COORDINATES filter
A COORDINATES filter SHALL take `operator` `overlaps` or `!overlaps` and a non-empty `values` list of one-based coordinate strings (`chr`, `chr:pos` or `chr:begin-end`). `overlaps` SHALL match a record that overlaps any listed region. `!overlaps` SHALL match a record that overlaps none of them. An unparseable coordinate SHALL fail the search with a `SearchError`.

#### Scenario: overlaps any
- **WHEN** the filter is `{"type":"COORDINATES","operator":"overlaps","values":["chr2:1000-2000","chr3"]}`
- **THEN** records on chr3, and records on chr2 overlapping 1000–2000, match

#### Scenario: !overlaps
- **WHEN** the filter is `{"type":"COORDINATES","operator":"!overlaps","values":["chr1","chr2:1000-2000"]}`
- **THEN** a record at `chr2:5000` matches, and records on chr1 or at `chr2:1500` do not

#### Scenario: Bad coordinate
- **WHEN** `values` contains `"chrZ:abc"`
- **THEN** the search fails with a `SearchError`

#### Scenario: Empty values
- **WHEN** `values` is `[]`
- **THEN** the search fails with a `SearchError`

### Requirement: GENES filter
A GENES filter SHALL take a non-empty `values` list of gene symbols. Each symbol is resolved to coordinates in the embedded gene database for the search's genome and gene source. A record matches when it overlaps any resolved gene. GENES SHALL fail with a `SearchError` if no genome is set, a symbol is not found, or gene lookup is unavailable.

#### Scenario: Gene panel
- **WHEN** the genome is `hg38`, gene source is `refseq`, and the filter is `{"type":"GENES","values":["BRCA1","BRCA2"]}`
- **THEN** only records overlapping BRCA1 or BRCA2 (by hg38 RefSeq coordinates) are returned

#### Scenario: Gene source defaults to RefSeq
- **WHEN** no gene source is given
- **THEN** genes are resolved from the RefSeq database

#### Scenario: No genome
- **WHEN** a GENES filter is used and the search has no genome
- **THEN** the search fails with a `SearchError`

#### Scenario: Unknown gene
- **WHEN** `values` contains `"NOTAGENE"`
- **THEN** the search fails with a `SearchError` naming the gene

### Requirement: Genome and gene source must be recognised for GENES
When a GENES filter is used, the search's genome SHALL be one of `hg19`, `hg38`, `grch37` or `grch38` (any case), and its gene source SHALL be `refseq` or `gencode`. Any other value SHALL fail the search with a `SearchError`; it SHALL NOT fall back to a default database.

#### Scenario: Unknown gene source
- **WHEN** a GENES filter is used with gene source `gencod`
- **THEN** the search fails with a `SearchError`

#### Scenario: Unknown genome
- **WHEN** a GENES filter is used with genome `hg37`
- **THEN** the search fails with a `SearchError`

#### Scenario: GENCODE source
- **WHEN** a GENES filter is used with genome `grch38` and gene source `gencode`
- **THEN** genes are resolved from the hg38 GENCODE database

### Requirement: Filter applies within the searched regions
When `SearchOptions.filter` is set, the search SHALL return, for each requested coordinate, only the VCF records that overlap that coordinate and match the filter. Records are returned in file order with one result list per coordinate, as an unfiltered search does. When `filter` is unset, results SHALL be identical to an unfiltered search.

#### Scenario: Per-coordinate results
- **WHEN** a filtered search runs on coordinates `chr1:1-1000000` and `chr2:1-1000000`
- **THEN** the result has two lists, each holding only that region's matching records

#### Scenario: No filter
- **WHEN** `filter` is not set
- **THEN** the result equals an unfiltered search's result

#### Scenario: Header unaffected
- **WHEN** a filtered search runs with the header requested
- **THEN** the full VCF header is returned unchanged

### Requirement: Filter only for VCF files
Setting a filter on a search whose output format is not VCF SHALL fail with a `SearchError` before any data is read.

#### Scenario: Filter on a BAM
- **WHEN** a filter is set on a search of a `.bam` file
- **THEN** the search fails with a `SearchError`

### Requirement: Validation happens before records are read
All filter checks that need no record data SHALL complete before any VCF records are fetched. This covers structure, operators, value types, the genotype-key rule, coordinate parsing, gene resolution and sample names (checked against the header).

#### Scenario: Unknown gene fails fast
- **WHEN** a filtered search has an unknown gene in a GENES filter
- **THEN** it fails with a `SearchError` without fetching any record data

### Requirement: CLI filter command
`seqa filter` SHALL take a file, one or more required coordinates, and `--spec <path>` to a JSON filter file. It SHALL also take the same `-r` reference, `-w` with-header, `-o` only-header and `--no-cache` options as `search`, plus `-s/--source` gene source (default `refseq`). Output SHALL be in the same format as `search`.

#### Scenario: Filtered output
- **WHEN** the user runs `seqa filter sample.vcf.gz chr1 --spec filter.json -r hg38`
- **THEN** the matching records for chr1 are printed in the same format `seqa search` uses

#### Scenario: Coordinates required
- **WHEN** the user runs `seqa filter sample.vcf.gz --spec filter.json`
- **THEN** argument parsing fails

#### Scenario: Unreadable or invalid spec file
- **WHEN** `--spec` points to a missing file or to invalid filter JSON
- **THEN** the command prints an error and exits with a non-zero status

### Requirement: HTTP filter endpoint
The axum server SHALL expose `POST /filter`. The body holds `path`, `coordinates`, `filter`, and optional `genome`, `gene_source`, `with_header` and `only_header`. It SHALL return the same JSON shape as `POST /search`. A missing or invalid filter, or a filter `SearchError`, SHALL return 400. Other search failures keep the `/search` status codes.

#### Scenario: Successful filter request
- **WHEN** a client posts `{"path":"s3://b/sample.vcf.gz","coordinates":["chr1"],"filter":{"type":"QUAL","operator":"gt","value":30},"genome":"hg38"}` to `/filter`
- **THEN** the response is 200 with `coordinates`, `header` and `lines`, where `lines` holds only matching records

#### Scenario: Missing filter
- **WHEN** the body has no `filter`
- **THEN** the response is 400

#### Scenario: Invalid filter
- **WHEN** the body's filter has an unknown operator
- **THEN** the response is 400

#### Scenario: Non-VCF path
- **WHEN** `path` is a `.bam` file
- **THEN** the response is 400

### Requirement: Lambda filter payload
The lambda search payload SHALL accept optional `filter` and `gene_source` fields. When `filter` is present, the search SHALL be filtered as `SearchOptions.filter` specifies. An invalid filter or a filter `SearchError` SHALL return 400. A payload without `filter` SHALL behave as before.

#### Scenario: Filtered lambda search
- **WHEN** a POST payload includes `"filter": {"type":"FILTERS","operator":"contains","value":"PASS"}`
- **THEN** the response's `lines` holds only PASS records

#### Scenario: Invalid filter in payload
- **WHEN** the payload's filter is `{"type":"INFO","operator":"gt","value":1}`, which is missing `key`
- **THEN** the response is 400

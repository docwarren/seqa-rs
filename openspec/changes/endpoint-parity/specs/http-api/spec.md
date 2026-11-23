# Spec Delta

## Purpose

Defines the HTTP endpoints that `seqa_axum` and `seqa_lambda` both serve. Each endpoint has the same path, request, response and status codes on both servers, so one UI client works against either.

## ADDED Requirements

### Requirement: Identical endpoint set
`seqa_axum` and `seqa_lambda` SHALL each serve exactly these routes: `POST /search`, `POST /files/list`, `GET /files/buckets/{backend}`, `GET /files/formats`, `GET /genes/symbols/{genome}/{source}`, `GET /cytobands/{genome}/{chromosome}`, `POST /notes/read` and `POST /notes/save`. Any other method and path SHALL return 404, except for `HEAD` and `OPTIONS` (see below).

#### Scenario: Unknown path
- **WHEN** a client sends `POST /anything` to either server
- **THEN** the response is 404

#### Scenario: Removed recursive listing
- **WHEN** a client sends `POST /files` to either server
- **THEN** the response is 404

#### Scenario: Old Lambda cytoband path
- **WHEN** a client sends `GET /genes/cytobands/grch38/chr1` to `seqa_lambda`
- **THEN** the response is 404

### Requirement: HEAD on GET routes is optional
A server MAY answer `HEAD` on a GET route with that route's status and headers and no body. A server that does not SHALL return the JSON 404. Clients SHALL NOT depend on `HEAD`.

#### Scenario: HEAD on axum
- **WHEN** a client sends `HEAD /files/formats` to `seqa_axum`
- **THEN** the response is 200 with an empty body

#### Scenario: HEAD on Lambda
- **WHEN** a client sends `HEAD /files/formats` to `seqa_lambda`
- **THEN** the response is 404 with a JSON error body

### Requirement: CORS preflight is not a route
An `OPTIONS` CORS preflight request SHALL be answered by the deployment's CORS handling (axum's CORS layer, or API Gateway for Lambda), not with the JSON 404. A preflight from an allowed origin SHALL be allowed.

#### Scenario: Preflight from the dev UI on axum
- **WHEN** the browser sends `OPTIONS /search` from `http://localhost:5173` with `Access-Control-Request-Method: POST`
- **THEN** axum responds 200 with `Access-Control-Allow-Origin: http://localhost:5173`

### Requirement: Trailing slash is optional on GET routes
Both servers SHALL treat a GET path with one trailing `/` the same as the path without it.

#### Scenario: Gene symbols with and without slash
- **WHEN** a client requests `/genes/symbols/grch38/refseq/` and then `/genes/symbols/grch38/refseq`
- **THEN** both return 200 with the same body

#### Scenario: Cytobands with slash
- **WHEN** a client requests `/cytobands/grch38/chr1/`
- **THEN** the response is 200 with chr1 cytobands

### Requirement: JSON error body
Every error response from either server SHALL have content type `application/json` and the body `{"error": "<message>"}`.

#### Scenario: Bad request on axum
- **WHEN** `seqa_axum` rejects a malformed `/search` body
- **THEN** the response is 400 with a JSON body that has an `error` string

#### Scenario: Not found on Lambda
- **WHEN** `seqa_lambda` receives an unknown path
- **THEN** the response is 404 with a JSON body that has an `error` string

### Requirement: Search request
`POST /search` SHALL accept `{"path", "coordinates", "with_header"?, "only_header"?, "index_path"?, "output_format"?}`. `index_path` replaces the inferred index, and `output_format` replaces the format inferred from the extension. Other fields, including `genome`, SHALL be ignored.

#### Scenario: Explicit index path
- **WHEN** a BAM is searched with `index_path` set to an index in another location
- **THEN** the search uses that index and returns 200

#### Scenario: Explicit output format
- **WHEN** a file with no recognized extension is searched with `"output_format": "bigwig"`
- **THEN** the file is searched as BigWig

#### Scenario: Coordinates must be a list
- **WHEN** a request sends `"coordinates": "chr1:1-1000"`
- **THEN** the response is 400

### Requirement: Search request validation
`POST /search` SHALL return 400 when:
- `coordinates` is empty (the error is `No coordinates`);
- a coordinate does not parse;
- the format cannot be determined from `output_format` or from the extension;
- `output_format` names an unknown format.

#### Scenario: Empty coordinates
- **WHEN** a request sends `"coordinates": []`
- **THEN** the response is 400 with error `No coordinates`

#### Scenario: Unsupported extension
- **WHEN** a request searches `s3://bucket/file.xyz` with no `output_format`
- **THEN** the response is 400

#### Scenario: Unknown output format
- **WHEN** a request sends `"output_format": "nonsense"`
- **THEN** the response is 400

### Requirement: Search response
`POST /search` SHALL return 200 with `{"coordinates", "header", "lines"}`, where `lines[i]` holds the results for `coordinates[i]` (empty when none). `header` SHALL hold the file's header lines when `with_header` or `only_header` is true, and be empty otherwise. A search that fails after validation SHALL return 500.

#### Scenario: Two coordinates
- **WHEN** a VCF is searched for a coordinate with records and then one without
- **THEN** `coordinates` lists both in order, and `lines` has two lists, the second empty

#### Scenario: Header requested
- **WHEN** a VCF is searched with `with_header` true
- **THEN** `header` includes the `#CHROM` line

#### Scenario: Header not requested
- **WHEN** the same search is sent without `with_header`
- **THEN** `header` is empty

### Requirement: Directory listing
`POST /files/list` SHALL accept `{"uri": "<uri>"}` and return 200 with a JSON array of `{"uri", "name", "isDirectory", "size", "lastModified"}`, one per object or sub-directory directly under `uri`. A store failure SHALL return 500, and a malformed body SHALL return 400.

#### Scenario: Listing a bucket prefix
- **WHEN** a client posts `{"uri": "s3://bucket/analysis/"}`
- **THEN** the response is 200 with the entries directly under that prefix, and each `uri` can be passed back to `/files/list`

#### Scenario: Malformed body
- **WHEN** a client posts a body with no `uri` field
- **THEN** the response is 400

#### Scenario: Form-encoded body
- **WHEN** a client posts `uri=s3://bucket/` as `application/x-www-form-urlencoded`
- **THEN** the response is 400

### Requirement: Listing rejects local paths
`POST /files/list` SHALL reject `file:` URIs and scheme-less paths with 400 without reading the server's filesystem.

#### Scenario: File URI
- **WHEN** a client posts `{"uri": "file:///etc"}`
- **THEN** the response is 400

#### Scenario: Scheme-less path
- **WHEN** a client posts `{"uri": "/etc/passwd"}`
- **THEN** the response is 400

### Requirement: Bucket listing
`GET /files/buckets/{backend}` SHALL return 200 with a JSON array of bucket or container names for `s3`, `gcs` or `azure`. Any other backend, including `local`, SHALL return 400. A listing failure SHALL return 500.

#### Scenario: Unknown backend
- **WHEN** a client requests `/files/buckets/nonsense`
- **THEN** the response is 400

#### Scenario: Local backend
- **WHEN** a client requests `/files/buckets/local`
- **THEN** the response is 400

### Requirement: File formats
`GET /files/formats` SHALL return 200 with the supported file format table as a JSON array.

#### Scenario: Formats listed
- **WHEN** a client requests `/files/formats`
- **THEN** the response is 200 and the array includes a BAM entry

### Requirement: Gene symbols
`GET /genes/symbols/{genome}/{source}` SHALL return 200 with the gene coordinates for that genome and source as a JSON array. An unknown genome SHALL fall back to hg38 and an unknown source SHALL fall back to RefSeq.

#### Scenario: RefSeq genes
- **WHEN** a client requests `/genes/symbols/grch38/refseq`
- **THEN** the response is 200 and includes `BRCA2`

#### Scenario: Unknown source falls back to RefSeq
- **WHEN** a client requests `/genes/symbols/grch38/nonsense`
- **THEN** the response is 200 with the same body as `/genes/symbols/grch38/refseq`

#### Scenario: Unknown genome falls back to hg38
- **WHEN** a client requests `/genes/symbols/nonsense/refseq`
- **THEN** the response is 200 with the same body as `/genes/symbols/hg38/refseq`

### Requirement: Cytobands
`GET /cytobands/{genome}/{chromosome}` SHALL return 200 with that chromosome's cytobands as a JSON array. An unknown genome SHALL fall back to hg38, and an unknown chromosome SHALL return an empty array.

#### Scenario: chr1 cytobands
- **WHEN** a client requests `/cytobands/grch38/chr1`
- **THEN** the response is 200 and every band is on chr1

#### Scenario: Unknown chromosome
- **WHEN** a client requests `/cytobands/grch38/chrNotReal`
- **THEN** the response is 200 with an empty array

#### Scenario: Unknown genome falls back to hg38
- **WHEN** a client requests `/cytobands/nonsense/chr1`
- **THEN** the response is 200 with the same body as `/cytobands/hg38/chr1`

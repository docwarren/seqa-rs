# Spec Delta

## Purpose

Lets the app read and save note files (text, images, audio, tables) kept in a `notes` directory next to analysis data. Every backend offers the same notes behavior.

## ADDED Requirements

### Requirement: Note paths must be inside a notes directory
A note path SHALL be accepted only if it is a `file://`, `s3://`, `gs://` or `az://` URI and its normalized object path has a directory segment named exactly `notes` before a non-empty file name. Any other path SHALL be rejected as invalid, and no object store SHALL be contacted.

#### Scenario: Cloud path inside a notes directory is accepted
- **WHEN** a note is requested at `s3://bucket/analysis/notes/today.txt`
- **THEN** the path is accepted

#### Scenario: Local path inside a notes directory passes the notes rule
- **WHEN** the notes rule checks `file:///home/user/analysis/notes/today.txt`
- **THEN** the path is accepted

#### Scenario: HTTP path is rejected
- **WHEN** a note is requested at `https://example.com/analysis/notes/today.txt`
- **THEN** the request is rejected as an invalid path

#### Scenario: Path outside a notes directory is rejected
- **WHEN** a note is requested at `s3://bucket/analysis/sample.vcf.gz`
- **THEN** the request is rejected as an invalid path

#### Scenario: Similarly named directory is rejected
- **WHEN** a note is requested at `s3://bucket/analysis/mynotes/today.txt`
- **THEN** the request is rejected as an invalid path

#### Scenario: Traversal out of a notes directory is rejected
- **WHEN** a note is requested at `s3://bucket/analysis/notes/../sample.vcf.gz`
- **THEN** the request is rejected as an invalid path

#### Scenario: Notes directory itself is rejected
- **WHEN** a note is requested at `s3://bucket/analysis/notes/`
- **THEN** the request is rejected as an invalid path

### Requirement: Servers reject local note paths
`seqa_axum` and `seqa_lambda` SHALL reject `file:` URIs and scheme-less paths on `/notes/read` and `/notes/save` with HTTP 400, even inside a notes directory.

#### Scenario: Local note over HTTP
- **WHEN** a client posts `{"path": "file:///home/user/analysis/notes/today.txt"}` to `/notes/read`
- **THEN** the response is 400

#### Scenario: Scheme-less note over HTTP
- **WHEN** a client posts `{"path": "/analysis/notes/today.txt"}` to `/notes/save`
- **THEN** the response is 400 and nothing is written

### Requirement: Read a note
`POST /notes/read` SHALL accept `{"path": "<uri>"}` and return HTTP 200 with `{"data": "<base64>"}`, where `data` is the object's full contents encoded as standard base64 with padding.

#### Scenario: Existing note is returned as base64
- **GIVEN** the object `s3://bucket/analysis/notes/today.txt` contains `Hello world`
- **WHEN** a client posts `{"path": "s3://bucket/analysis/notes/today.txt"}` to `/notes/read`
- **THEN** the response is 200 with `{"data": "SGVsbG8gd29ybGQ="}`

#### Scenario: Missing note
- **WHEN** a client reads a valid note path that has no object
- **THEN** the response is 404

#### Scenario: Invalid note path on read
- **WHEN** a client reads a path that fails the note path rules
- **THEN** the response is 400

#### Scenario: Malformed read body
- **WHEN** a client posts a body that is not JSON or has no `path` field to `/notes/read`
- **THEN** the response is 400

### Requirement: Save a note
`POST /notes/save` SHALL accept `{"path": "<uri>", "data": "<base64>"}`, decode `data`, write the bytes to `path` (creating or overwriting the object), and return HTTP 204 with an empty body.

#### Scenario: Note is saved and can be read back
- **WHEN** a client saves `{"path": "s3://bucket/analysis/notes/today.txt", "data": "SGVsbG8gd29ybGQ="}`
- **THEN** the response is 204
- **AND** a later read of that path returns `{"data": "SGVsbG8gd29ybGQ="}`

#### Scenario: Existing note is overwritten
- **GIVEN** a note already exists at a valid path
- **WHEN** a client saves new data to that path
- **THEN** a later read returns the new data

#### Scenario: Invalid note path on save
- **WHEN** a client saves to a path that fails the note path rules
- **THEN** the response is 400 and nothing is written

#### Scenario: Invalid base64
- **WHEN** a client saves with `data` that is not valid standard base64
- **THEN** the response is 400 and nothing is written

#### Scenario: Malformed save body
- **WHEN** a client posts a body that is not JSON or has no `path` or `data` field to `/notes/save`
- **THEN** the response is 400

#### Scenario: Form-encoded save is rejected
- **WHEN** a client posts `path=...&data=...` as `application/x-www-form-urlencoded` to `/notes/save`
- **THEN** the response is 400 and nothing is written

### Requirement: Note store failures are server errors
A note read or save that passes validation but fails in the object store for a reason other than a missing object SHALL return HTTP 500 with a message that names the path.

#### Scenario: Store rejects the write
- **WHEN** a save to a valid note path fails in the object store (for example, access denied)
- **THEN** the response is 500 and the error message contains the path

# lambda-search Specification

## Purpose

Defines the search request and response of the `seqa_lambda` HTTP function, so it accepts and returns the same shapes as `seqa_axum` and the Tauri backend.

## Requirements

### Requirement: Search takes a list of coordinates
A search request SHALL carry `coordinates` as a JSON list of coordinate strings, and the lambda SHALL search the file for every one of them in a single request.

#### Scenario: Two coordinates
- **WHEN** a VCF is searched with `"coordinates": ["chr1:116549-116549", "chr1:1-2"]`
- **THEN** the response is 200 and `coordinates` lists both, in the order sent

#### Scenario: Single string rejected
- **WHEN** a request sends `"coordinates": "chr1:1-1000"`
- **THEN** the response is 400

### Requirement: Empty coordinates rejected
A search request with an empty `coordinates` list SHALL be rejected with status 400 and the error `No coordinates`.

#### Scenario: Empty list
- **WHEN** a request sends `"coordinates": []`
- **THEN** the response is 400 and its body holds the error `No coordinates`

### Requirement: Results paired with coordinates
The response SHALL be `{ coordinates, header, lines }`, with one list in `lines` per entry in `coordinates`, so `lines[i]` holds the results for `coordinates[i]`. A coordinate with no results SHALL still have an entry, which is empty.

#### Scenario: Second region has no records
- **WHEN** a VCF is searched for a coordinate with records and then one without
- **THEN** `lines` has two lists: the first isn't empty and the second is

### Requirement: Header only when asked
The response `header` SHALL hold the file's header lines when `with_header` or `only_header` is true, and SHALL be empty otherwise.

#### Scenario: Header requested
- **WHEN** a BAM is searched with `with_header` true
- **THEN** `header` isn't empty and `lines` has one list per coordinate

#### Scenario: Header not requested
- **WHEN** the same search is sent without `with_header`
- **THEN** `header` is empty

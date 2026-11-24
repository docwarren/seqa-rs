# seqa_obos

Load an OBO ontology from S3, Azure, GCS, HTTP(S) or local disk, keep it in memory, and search its terms. `seqa_obos` is a Rust library for web servers, Tauri apps and Lambdas, plus an `obos` command-line tool built on the same API. It works with any OBO ontology (HPO, MONDO, GO, ...).

## Library

```rust
#[tokio::main]
async fn main() -> Result<(), seqa_obos::Error> {
    seqa_obos::load("s3://my-bucket/hp.obo").await?;

    let results = seqa_obos::search("microceph", &seqa_obos::SearchOptions::default())?;
    for term in &results {
        println!("{} {}", term.id, term.name);
    }

    // Results are full term entries and implement serde::Serialize.
    println!("{}", serde_json::to_string(&results).unwrap());
    Ok(())
}
```

- `load(location)` is async and must be awaited inside a Tokio runtime (as `#[tokio::main]` provides). It fetches, parses and indexes the file, and keeps it as the process's single active ontology. Loading the same location again does nothing; loading a different location replaces it. If a load fails, the previous ontology stays active. If loads of two different locations run at the same time, the one that finishes last wins.
- `search(query, &options)` is synchronous and runs on the in-memory index. It returns `Vec<Term>`, where each `Term` is the full `[Term]` entry (ID, name, alt IDs, definition, synonyms, xrefs, `is_a` and every other clause, with qualifiers and trailing comments).
- `SearchOptions::default()` matches synonyms and returns at most 100 results. A `limit` of 0 is an error.
- `is_query_too_short(query)` tells you whether a query will be ignored (see below). `search` returns an empty list for such queries, so it is safe to call on every keystroke.
- `Error`, `Term` and the clause structs are `#[non_exhaustive]`, so new variants and fields can be added without a breaking change. Match on `Error` with a `_` arm.

## CLI

The CLI is behind the `cli` feature, so library users don't pull in `clap`:

```sh
cargo build --release -p seqa_obos --features cli      # target/release/obos
cargo install --path seqa_obos --features cli          # or install it
```

Usage:

```
obos search [--no-synonyms] [--limit N] [--json] <LOCATION> <QUERY>
```

By default each result is printed as an OBO `[Term]` stanza, with a blank line between results:

```console
$ obos search ./tests/fixtures/mini.obo 'progressive micr'
[Term]
id: HP:0000253
name: Progressive microcephaly
is_a: HP:0000252 ! Microcephaly

[Term]
id: HP:0000255
name: Microcephaly, progressive
is_a: HP:0000252 ! Microcephaly

[Term]
id: HP:0000254
name: Progressive congenital microcephaly
is_a: HP:0000253 ! Progressive microcephaly
```

With `--json` the results are printed as a JSON array of term objects (`[]` when nothing matches):

```console
$ obos search --limit 1 --json ./tests/fixtures/mini.obo hp:0001366
[{"id":"HP:0000252","name":"Microcephaly","def":{"text":"Head circumference below 2 standard deviations below the mean for age and gender.","xrefs":[{"id":"HPO:probinson"},{"id":"PMID:19125436"}]},"alt_ids":[{"id":"HP:0001366"}],"synonyms":[{"text":"Small head","scope":"EXACT","type":"layperson","xrefs":[{"id":"ORCID:0000-0001-5208-3432"}]},{"text":"Microcephalus","scope":"EXACT","xrefs":[]},{"text":"Severe microcephaly","scope":"NARROW","xrefs":[]},{"text":"Reduced braincase","scope":"BROAD","xrefs":[]},{"text":"Braincase anomaly","scope":"RELATED","xrefs":[]}],"xrefs":[{"id":"UMLS:C0025958","qualifiers":[{"key":"source","value":"MONDO"}],"comment":"from MONDO"},{"id":"SNOMEDCT_US:1829003"}],"is_a":[{"id":"HP:0040195","comment":"Decreased head circumference"}]}]
```

EXACT synonyms are matched by default. Here HP:0000252 is found through its EXACT synonym "Small head":

```console
$ obos search ./tests/fixtures/mini.obo 'small head'
[Term]
id: HP:0040196
name: Small head circumference
is_a: HP:0040195 ! Decreased head circumference

[Term]
id: HP:0000252
name: Microcephaly
alt_id: HP:0001366
def: "Head circumference below 2 standard deviations below the mean for age and gender." [HPO:probinson, PMID:19125436]
synonym: "Small head" EXACT layperson [ORCID:0000-0001-5208-3432]
synonym: "Microcephalus" EXACT []
synonym: "Severe microcephaly" NARROW []
synonym: "Reduced braincase" BROAD []
synonym: "Braincase anomaly" RELATED []
xref: UMLS:C0025958 {source="MONDO"} ! from MONDO
xref: SNOMEDCT_US:1829003
is_a: HP:0040195 ! Decreased head circumference
```

`--no-synonyms` matches only IDs, alt IDs and names:

```console
$ obos search --no-synonyms ./tests/fixtures/mini.obo 'small head'
[Term]
id: HP:0040196
name: Small head circumference
is_a: HP:0040195 ! Decreased head circumference
```

Queries that are too short are not searched, and the file is not loaded. With `--json`, stdout is `[]` and the message goes to stderr:

```console
$ obos search ./tests/fixtures/mini.obo hp
Search queries must contain at least 3 letters or digits.
```

Exit codes: 0 on success (including no matches), 1 if loading or searching fails (the error goes to stderr), 2 for usage errors such as a missing argument or `--limit 0`.

## What is matched

Only named, non-obsolete `[Term]` frames are searched. Matching ignores case.

- **ID and alt ID:** the whole query, trimmed, matches when it is a prefix of the term's `id` or one of its `alt_id`s. `hp:000025` matches HP:0000250 to HP:0000259; `0000252` does not match HP:0000252.
- **Name:** the query and the name are split into words at every non-alphanumeric character. The name matches when every query word is the start of some word in the name, in any order. `progressive micr` matches "Progressive microcephaly" and "Microcephaly, progressive".
- **Synonyms:** EXACT synonyms are matched with the same word rules. This is on by default; turn it off with `SearchOptions { include_synonyms: false, .. }` or the CLI's `--no-synonyms`. NARROW, BROAD and RELATED synonyms are never matched, but every synonym is included in the results.

Queries need at least 3 letters or digits; spaces and punctuation don't count, so `hp`, `hp:` and `a b` return nothing.

### Ranking

Each term is listed once, at its best match:

1. Exact ID
2. ID prefix
3. Exact alt ID
4. Alt ID prefix
5. Exact name
6. Name prefix
7. Exact synonym (EXACT scope only)
8. Synonym prefix (EXACT scope only)
9. Any other word match on the name or an EXACT synonym

Ties are sorted by name (ignoring case), then ID. Results are capped by the limit (default 100).

## Locations and credentials

| Store | Location |
|---|---|
| Amazon S3 | `s3://bucket/path/hp.obo` |
| Azure Blob Storage | `az://`, `abfs://`, `abfss://`, `azure://` |
| Google Cloud Storage | `gs://bucket/path/hp.obo` |
| HTTP(S) | `https://example.org/hp.obo` |
| Local disk | `file:///abs/path/hp.obo`, `/abs/path/hp.obo` or `./relative/hp.obo` |

Credentials and store configuration come only from:

- the provider's environment variables: `AWS_*` for S3 (e.g. `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `AWS_REGION`), `AZURE_*` for Azure (e.g. `AZURE_STORAGE_ACCOUNT_NAME`), `GOOGLE_*` for GCS (e.g. `GOOGLE_SERVICE_ACCOUNT`);
- platform roles and metadata services: an EC2 instance role, a Lambda execution role, ECS task credentials, or web identity.

Nothing is passed through the API. All other environment variables are ignored, so a generic `TOKEN`, `REGION`, `ENDPOINT` or `ALLOW_INVALID_CERTIFICATES` has no effect.

Make sure the right variables are set in the environment of the process that calls `load` (or runs `obos`). In a shell, that means `export`ing them; a plain `VAR=value` is not passed on to programs.

### Amazon S3

- **Set `AWS_REGION` to the bucket's region.** The region is not detected automatically: if it is missing, `us-east-1` is used, and a bucket in another region fails with `Received redirect without LOCATION, this normally indicates an incorrectly configured region`. Find a bucket's region with `aws s3api get-bucket-location --bucket BUCKET` (`null` means `us-east-1`). `AWS_DEFAULT_REGION` also works.
- **Set credentials** with `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` (plus `AWS_SESSION_TOKEN` for temporary credentials), unless the code runs with a role (EC2, Lambda, ECS, web identity).
- **AWS profiles (`~/.aws/config`, `~/.aws/credentials`, `AWS_PROFILE`) are not read.** To use a profile locally, export its credentials into the environment:

```sh
eval "$(aws configure export-credentials --format env)"   # add --profile NAME if needed
export AWS_REGION=eu-west-2                               # the bucket's region
```

### Azure Blob Storage

Set `AZURE_STORAGE_ACCOUNT_NAME`, plus one way to authenticate: `AZURE_STORAGE_ACCOUNT_KEY`, a SAS token in `AZURE_STORAGE_SAS_KEY`, or a service principal (`AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`). On Azure compute, a managed identity is used when no key is set.

### Google Cloud Storage

Set `GOOGLE_SERVICE_ACCOUNT` to the path of a service-account JSON key file (or `GOOGLE_SERVICE_ACCOUNT_KEY` to its contents). Without either, the gcloud application default credentials (`gcloud auth application-default login`, stored in `~/.config/gcloud/`) are used if present, and on Google Cloud the instance's service account.

## Size limit and memory

Files larger than 4 GB (4,294,967,296 bytes) are rejected before they are downloaded. While a file is loading, its bytes and its parsed form are both in memory, so peak memory is several times the file size; after indexing only the term index is kept. Real ontologies are far below the limit (HPO is about 11 MB).

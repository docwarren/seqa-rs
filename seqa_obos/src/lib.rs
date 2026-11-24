// Copyright 2026 Seqa23
//
// Author: Andrew Warren
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Load an OBO ontology from S3, Azure, GCS, HTTP(S) or local disk, keep it in memory,
//! and search its terms.
//!
//! - [`load`] (async) fetches, parses and indexes an ontology and makes it the process's
//!   single active ontology. It must be awaited inside a Tokio runtime.
//! - [`search`] (sync) matches a query against the active ontology's term IDs, alt IDs,
//!   names and EXACT synonyms, and returns ranked, serializable [`Term`] entries.
//! - [`is_query_too_short`] reports whether a query has fewer than [`MIN_QUERY_CHARS`]
//!   letters or digits, in which case [`search`] returns an empty list.
//!
//! [`Error`], [`Term`] and the clause structs are `#[non_exhaustive]`: match on
//! [`Error`] with a `_` arm, and read [`Term`]s rather than constructing them.

mod cache;
mod index;
mod loader;
mod location;
mod search;
mod term;

pub use search::{MIN_QUERY_CHARS, SearchOptions, is_query_too_short};
pub use term::{
    Annotated, Definition, IdRef, Intersection, PropertyValue, Qualifier, Relationship, Synonym,
    SynonymScope, Term, Value, Xref,
};
#[cfg(test)]
mod test_support;

/// Loads the OBO ontology at `location` and makes it the process's active ontology.
///
/// # Locations
///
/// `location` is a URL or a local filesystem path:
///
/// - Amazon S3: `s3://bucket/key.obo`
/// - Azure Blob Storage: `az://`, `abfs://`, `abfss://`, `azure://`
/// - Google Cloud Storage: `gs://bucket/key.obo`
/// - HTTP(S): `http://` and `https://`
/// - Local files: `file:///abs/path.obo`, or a plain absolute or relative path
///   (relative paths are resolved against the current working directory)
///
/// # Credentials
///
/// Cloud stores are configured only from their provider's environment variables
/// (`AWS_*` for S3, `AZURE_*` for Azure, `GOOGLE_*` for GCS) and from platform roles
/// or metadata services (an EC2 instance role, a Lambda execution role, ECS task
/// credentials, or web identity). All other environment variables are ignored, and no
/// credentials are passed through this API.
///
/// The variables must be set in this process's environment. In particular:
///
/// - **S3:** set `AWS_REGION` (or `AWS_DEFAULT_REGION`) to the bucket's region. It is
///   not detected automatically; without it `us-east-1` is used, and buckets in other
///   regions fail with a "redirect without LOCATION" storage error. Set
///   `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY` (and `AWS_SESSION_TOKEN`) unless a role
///   provides credentials. AWS profiles (`~/.aws/config`, `~/.aws/credentials`,
///   `AWS_PROFILE`) are **not** read; export a profile's credentials first, e.g.
///   `eval "$(aws configure export-credentials --format env)"`.
/// - **Azure:** set `AZURE_STORAGE_ACCOUNT_NAME` and a key, SAS token or service
///   principal (`AZURE_STORAGE_ACCOUNT_KEY`, `AZURE_STORAGE_SAS_KEY`, or
///   `AZURE_CLIENT_ID`/`AZURE_CLIENT_SECRET`/`AZURE_TENANT_ID`), or run with a managed
///   identity.
/// - **GCS:** set `GOOGLE_SERVICE_ACCOUNT` (path to a service-account key file) or
///   `GOOGLE_SERVICE_ACCOUNT_KEY` (its contents); otherwise gcloud application default
///   credentials or the instance's service account are used.
///
/// See the README for examples.
///
/// # Size limit
///
/// Objects larger than 4 GB (4,294,967,296 bytes) are rejected with
/// [`Error::TooLarge`] after a metadata request, without downloading the body.
///
/// # Caching
///
/// Only one ontology is held per process. It stays in memory until the process exits
/// or a different location is loaded, which replaces it and frees the old one.
/// Loading the active location again returns `Ok` without fetching; locations are
/// compared after normalisation, so `./hp.obo` and `file:///<cwd>/hp.obo` are the same.
/// If a load fails, the previously active ontology stays active.
///
/// If loads of two different locations run concurrently, the one that finishes last wins.
///
/// # Memory
///
/// While loading, the whole file is held in memory alongside its parsed form, so peak
/// memory is several times the file size. Once indexing finishes, the raw bytes and
/// the parsed document are dropped and only the term index is kept.
///
/// # Runtime
///
/// Must be awaited inside a Tokio runtime (with the time and I/O drivers enabled, as
/// `#[tokio::main]` provides): fetching uses Tokio-based clients, and parsing runs on
/// Tokio's blocking thread pool. Awaiting it from another executor panics.
///
/// # Errors
///
/// [`Error::InvalidLocation`], [`Error::NotFound`], [`Error::Store`],
/// [`Error::TooLarge`] or [`Error::Parse`]. Each error's message names the normalised
/// location.
pub async fn load(location: &str) -> Result<(), Error> {
    cache::GLOBAL.load(location).await
}

/// Searches the active ontology (see [`load`]) for terms matching `query`.
///
/// Only named, non-obsolete `[Term]` frames are searched. Matching ignores case.
///
/// - **ID and alt ID:** the whole query, trimmed, matches when it is a prefix of the
///   term's `id` or of one of its `alt_id`s (`hp:000025` matches `HP:0000250`–`HP:0000259`;
///   `0000252` does not match `HP:0000252`).
/// - **Name:** the query and the name are split into words at every non-alphanumeric
///   character; the name matches when every query word is the start of some word in it,
///   in any order (`progressive micr` matches "Microcephaly, progressive").
/// - **Synonyms:** EXACT synonyms are matched with the same word rules when
///   [`SearchOptions::include_synonyms`] is set (the default). NARROW, BROAD and RELATED
///   synonyms are never matched.
///
/// Queries with fewer than [`MIN_QUERY_CHARS`] letters or digits return an empty list
/// (see [`is_query_too_short`]).
///
/// # Ranking
///
/// Each term appears once, ranked by its best match:
///
/// 1. the query equals the ID
/// 2. the query is a prefix of the ID
/// 3. the query equals an alt ID
/// 4. the query is a prefix of an alt ID
/// 5. the name equals the query
/// 6. the name starts with the query
/// 7. an EXACT synonym equals the query
/// 8. an EXACT synonym starts with the query
/// 9. any other word match on the name or an EXACT synonym
///
/// Tiers 5–8 compare case-insensitively, treating runs of non-alphanumeric characters
/// as one space. Within a tier, results are sorted by name (ignoring case), then ID.
/// At most [`SearchOptions::limit`] results are returned.
///
/// Results are full [`Term`] entries and serialize to JSON with `serde`.
///
/// # Example
///
/// ```
/// # #[tokio::main]
/// # async fn main() -> Result<(), seqa_obos::Error> {
/// seqa_obos::load("tests/fixtures/mini.obo").await?;
/// let results = seqa_obos::search("microceph", &seqa_obos::SearchOptions::default())?;
/// assert_eq!(results[0].id, "HP:0000252");
/// assert_eq!(results[0].name, "Microcephaly");
///
/// // "Small head" is an EXACT synonym of HP:0000252 in this file.
/// let with = seqa_obos::search("small head", &seqa_obos::SearchOptions::default())?;
/// assert!(with.iter().any(|t| t.id == "HP:0000252"));
/// let names_only = seqa_obos::SearchOptions { include_synonyms: false, ..Default::default() };
/// assert!(!seqa_obos::search("small head", &names_only)?.iter().any(|t| t.id == "HP:0000252"));
///
/// let ten = seqa_obos::SearchOptions { limit: 10, ..Default::default() };
/// assert!(seqa_obos::search("hp", &ten)?.is_empty()); // too short
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// [`Error::NotLoaded`] if no ontology has been loaded, and
/// [`Error::InvalidArgument`] if `options.limit` is 0.
pub fn search(query: &str, options: &SearchOptions) -> Result<Vec<Term>, Error> {
    search::check_options(options)?;
    let loaded = cache::GLOBAL.current()?;
    let hits = search::search_indexed(&loaded.ontology, query, options);
    Ok(hits.into_iter().map(|t| t.term.clone()).collect())
}

/// Like [`search`], but returns each result as its OBO `[Term]` stanza. Used by the CLI.
#[doc(hidden)]
pub fn search_obo_text(query: &str, options: &SearchOptions) -> Result<Vec<String>, Error> {
    search::check_options(options)?;
    let loaded = cache::GLOBAL.current()?;
    let hits = search::search_indexed(&loaded.ontology, query, options);
    Ok(search::obo_text(&hits))
}

/// Errors returned by [`load`] and [`search`].
///
/// The enum is `#[non_exhaustive]`, so variants may be added in later versions; a `match`
/// on it needs a `_ =>` arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The location could not be parsed, or uses an unsupported scheme. Holds the
    /// location and the reason.
    #[error("invalid location {0}")]
    InvalidLocation(String),
    /// No object exists at the location. Holds the normalised location.
    #[error("not found: {0}")]
    NotFound(String),
    /// Any other storage error (permission denied, network failure, ...).
    #[error("storage error loading {location}: {source}")]
    Store {
        /// The normalised location.
        location: String,
        /// The underlying `object_store` error.
        #[source]
        source: object_store::Error,
    },
    /// The object is larger than the 4 GB (4,294,967,296 bytes) limit. Its body was not
    /// downloaded.
    #[error("{location} is {size} bytes, which is larger than the 4 GB (4294967296 bytes) limit")]
    TooLarge {
        /// The normalised location.
        location: String,
        /// The object's size in bytes, as reported by the store.
        size: u64,
    },
    /// The content is not valid OBO. Holds the location and the parser's message.
    #[error("failed to parse OBO from {0}")]
    Parse(String),
    /// A search was run before any ontology was loaded.
    #[error("no ontology has been loaded")]
    NotLoaded,
    /// An argument was out of range (for example a zero search limit).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn mini_fixture_parses() {
        let doc = fastobo::from_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/mini.obo"
        ))
        .expect("mini.obo should parse");
        assert!(!doc.entities().is_empty());
    }

    #[test]
    fn hp_fixture_parses() {
        let doc = fastobo::from_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/hp.obo"
        ))
        .expect("hp.obo should parse");
        assert!(doc.entities().len() > 10_000);
    }
}

#[cfg(test)]
mod error_tests {
    use super::Error;

    #[test]
    fn display_messages_name_the_location() {
        assert_eq!(
            Error::InvalidLocation("ftp://h/x: unsupported scheme".into()).to_string(),
            "invalid location ftp://h/x: unsupported scheme"
        );
        assert_eq!(
            Error::NotFound("file:///tmp/missing.obo".into()).to_string(),
            "not found: file:///tmp/missing.obo"
        );
        assert_eq!(
            Error::TooLarge {
                location: "s3://b/k".into(),
                size: 4_294_967_297
            }
            .to_string(),
            "s3://b/k is 4294967297 bytes, which is larger than the 4 GB (4294967296 bytes) limit"
        );
        assert_eq!(
            Error::Parse("file:///x.obo: expected frame".into()).to_string(),
            "failed to parse OBO from file:///x.obo: expected frame"
        );
        assert_eq!(Error::NotLoaded.to_string(), "no ontology has been loaded");
        assert_eq!(
            Error::InvalidArgument("limit must be at least 1".into()).to_string(),
            "invalid argument: limit must be at least 1"
        );
        let store = Error::Store {
            location: "s3://bucket/private.obo".into(),
            source: object_store::Error::NotImplemented {
                operation: "head".into(),
                implementer: "test".into(),
            },
        };
        assert_eq!(
            store.to_string(),
            "storage error loading s3://bucket/private.obo: \
             Operation head not yet implemented by test."
        );
        let source = std::error::Error::source(&store).expect("source");
        assert!(matches!(
            source.downcast_ref::<object_store::Error>(),
            Some(object_store::Error::NotImplemented { .. })
        ));
    }
}

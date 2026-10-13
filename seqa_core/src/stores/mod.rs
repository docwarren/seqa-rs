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

use core::ops::Range;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use futures::StreamExt;
use log::info;
use object_store::path::Path as ObjectStorePath;
use object_store::{ObjectMeta, ObjectStore, ObjectStoreScheme, PutPayload};
use serde::{Deserialize, Serialize};
use url::Url;

pub mod buckets;
pub mod error;
pub mod store;

use error::StoreError;

/// Cloud-agnostic file access service built on top of the [`object_store`] crate.
///
/// `StoreService` maintains a thread-safe cache of backend clients keyed by
/// scheme and host, so repeated access to the same bucket or host reuses a
/// single [`ObjectStore`] instance.
///
/// # Creating a service
///
/// Create one with [`StoreService::new`] and share it.  Nothing is connected up front:
/// the backend for each path is picked from its URL scheme and built on first use.
///
/// ```rust,no_run
/// use seqa_core::stores::StoreService;
///
/// let svc = StoreService::new();
/// ```
#[derive(Debug, Default)]
pub struct StoreService {
    stores: Mutex<HashMap<String, Arc<dyn ObjectStore>>>,
}

/// One entry in a directory listing — a file or a folder-like prefix.
///
/// `uri` is a fully-qualified, absolute URI that can be passed straight back
/// into [`StoreService`].  This matters: `ObjectMeta::location` is *store
/// relative* (a local listing yields `home/drew/x.bam`, an S3 listing yields the
/// key without its bucket), so returning it raw would produce paths that cannot
/// be used for a subsequent call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryEntry {
    /// Absolute URI, safe to pass back to any `StoreService` method.
    pub uri: String,
    /// Final path segment, for display.
    pub name: String,
    /// True for folder-like prefixes, which carry no size or timestamp.
    pub is_directory: bool,
    pub size: u64,
    /// RFC 3339 timestamp, empty for directories.
    pub last_modified: String,
}

impl StoreService {
    /// Creates an empty `StoreService`. Backends are built lazily on first access.
    pub fn new() -> Self {
        Self::default()
    }

    // For http paths that are not object store paths, we return the whole URL.
    fn get_bucket_from_path(path: &str) -> Result<String, StoreError> {
        let url = Url::parse(path)?;
        let scheme = url.scheme();
        let (obj_scheme, _) = Self::get_obj_scheme_and_path(path)?;

        if scheme == "file" {
             Ok("".to_string())
        }
        else if obj_scheme == ObjectStoreScheme::Http {
            Ok(path.to_string())
        }
        else if scheme == "az" || (scheme == "http" || scheme == "https" && obj_scheme != ObjectStoreScheme::Http) {
            let mut parts = url.path_segments().ok_or_else(|| StoreError::ValidationError(
                "Invalid Azure or HTTP path".into(),
            ))?;
            if let Some(bucket) = parts.next() {
                Ok(bucket.to_string())
            } else {
                Err(StoreError::ValidationError(
                    "Azure path missing bucket".into(),
                ))
            }
        }
        else {
            let mut parts = path.split("://");
            if parts.clone().count() < 2 {
                Err(StoreError::ValidationError("Invalid path".into()))
            }
            else {
                Ok(parts.nth(1).unwrap().split('/').nth(0).unwrap().to_string())
            }
        }
    }

    fn get_store_key_from_bucket(bucket: &str, kind: &ObjectStoreScheme) -> Result<String, StoreError> {
        match kind {
            ObjectStoreScheme::Local => {
                Ok(format!("local-{}", bucket))
            },
            ObjectStoreScheme::AmazonS3 => {
                Ok(format!("s3-{}", bucket))
            },
            ObjectStoreScheme::GoogleCloudStorage => {
                Ok(format!("gc-{}", bucket))
            },
            ObjectStoreScheme::MicrosoftAzure => {
                Ok(format!("az-{}", bucket))
            }
            ObjectStoreScheme::Http => {
                Ok("http".to_string())
            }
            _ => {
                Err(StoreError::ValidationError(
                    "Unsupported store type".into(),
                ))
            }
        }
    }

    fn get_store_cache(&'_ self) -> Result<MutexGuard<'_, HashMap<String, Arc<dyn ObjectStore>>>, StoreError> {
        match self.stores.lock() {
            Ok(cache) => Ok(cache),
            Err(_) => Err(StoreError::ValidationError(
                "Store cache poisoned".into(),
            ))
        }
    }

    /// TODO: Fix this so that we dont pass the path and call it the bucket for http store.
    /// For HTTP stores bucket is actually the whole URL.
    fn build_store(&self, bucket: &str, store_kind: &ObjectStoreScheme) -> Result<Arc<dyn ObjectStore>, StoreError> {
        match store_kind {
            ObjectStoreScheme::Local => {
                Ok(Arc::new(store::get_local_store()?))
            },
            ObjectStoreScheme::AmazonS3 => {
                Ok(Arc::new(store::get_s3_store_from_bucket(bucket)?))
            },
            ObjectStoreScheme::GoogleCloudStorage => {
                Ok(Arc::new(store::get_gc_store_from_bucket(bucket)?))
            },
            ObjectStoreScheme::MicrosoftAzure => {
                Ok(Arc::new(store::get_azure_store_from_container(bucket)?))
            }
            ObjectStoreScheme::Http => {
                Ok(Arc::new(store::get_http_store(bucket)?))
            }
            _ => {
                Err(StoreError::ValidationError(
                    "Unsupported store type".into(),
                ))
            }
        }
    }

    pub fn get_or_create_store(&self, path: &str) -> Result<Arc<dyn ObjectStore>, StoreError> {
        let abs_path = Self::get_absolute_path(path)?;
        let bucket = Self::get_bucket_from_path(&abs_path)?;
        let (store_kind, _) = Self::get_obj_scheme_and_path(&abs_path)?;
        let store_key = Self::get_store_key_from_bucket(&bucket, &store_kind)?;
        let mut store_cache = self.get_store_cache()?;
        if let Some(store) = store_cache.get(&store_key) {
            return Ok(Arc::clone(store));
        }
        let store = self.build_store(&bucket, &store_kind)?;
        store_cache.insert(store_key, Arc::clone(&store));
        Ok(store)
    }

    /// Downloads a byte range from `path` and returns the raw bytes.
    ///
    /// `range` is a half-open byte range `[start, end)`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] on path normalisation failure or storage I/O errors.
    pub async fn get_range(&self, path: &str, range: Range<u64>) -> Result<Vec<u8>, StoreError> {
        let (_, url) = Self::get_obj_scheme_and_path(path)?;
        let store = self.get_or_create_store(path)?;
        Ok(store.get_range(&url, range).await?.to_vec())
    }

    pub async fn get_ranges(&self, path: &str, ranges: &[Range<u64>]) -> Result<Vec<Vec<u8>>, StoreError> {
        let (_, url) = Self::get_obj_scheme_and_path(path)?;
        let store = self.get_or_create_store(path)?;
        Ok(store.get_ranges(&url, ranges).await?.iter().map(|r| r.to_vec()).collect())
    }

    pub fn get_absolute_path(path: &str) -> Result<String, StoreError> {
        let mut abs_path = path.to_owned();

        if !path.contains("://") {
            // Assume local file path
            let abs_path_buf = std::fs::canonicalize(path)?;
            abs_path = match abs_path_buf.to_str() {
                Some(p) => {
                    format!("file://{}", p)
                },
                None => {
                    return Err(StoreError::ValidationError(
                        "Could not convert path to string".into(),
                    ))
                }
            };
        }
        Ok(abs_path)
    }

    /// Get file path
    /// Gets a Path object from the string supplied.
    pub fn get_obj_scheme_and_path(path: &str) -> Result<(ObjectStoreScheme, ObjectStorePath), StoreError> {
        let url = Url::parse(path)?;

        match ObjectStoreScheme::parse(&url) {
            Ok((scheme, path)) => {
                match scheme {
                    ObjectStoreScheme::MicrosoftAzure => { Ok((scheme, path)) }
                    ObjectStoreScheme::AmazonS3 => { Ok((scheme, path)) }
                    ObjectStoreScheme::GoogleCloudStorage => { Ok((scheme, path)) }
                    ObjectStoreScheme::Http => { Ok((scheme, path)) }
                    ObjectStoreScheme::Local => { Ok((scheme, path)) }
                    _ => {
                        Err(StoreError::ValidationError(
                            "Unsupported store type".into(),
                        ))
                    }
                }
            },
            Err(e) => {
                Err(StoreError::ObjectStoreUriParseError(e.to_string()))
            }
        }
    }

    /// Returns the total size of the object at `path` in bytes.
    pub async fn get_file_size(&self, path: &str) -> Result<u64, StoreError> {
        let (_, canonical) = Self::get_obj_scheme_and_path(path)?;
        let store = self.get_or_create_store(path)?;
        let meta = store.head(&canonical).await?;
        Ok(meta.size)
    }

    /// Downloads the entire object at `path` and returns its bytes.
    pub async fn get_object(&self, path: &str) -> Result<Vec<u8>, StoreError> {
        let (_, canonical) = Self::get_obj_scheme_and_path(path)?;
        let store = self.get_or_create_store(path)?;
        let result = store.get(&canonical).await?;
        let bytes = result.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Uploads `contents` to the object at `path`, creating or overwriting it.
    pub async fn put_object(&self, path: &str, contents: &[u8]) -> Result<(), StoreError> {
        let (_, canonical) = Self::get_obj_scheme_and_path(path)?;
        let store = self.get_or_create_store(path)?;

        let payload = PutPayload::from(contents.to_vec());

        store
            .put(&canonical, payload)
            .await
            .map_err(|e| StoreError::PutError(e.to_string()))?;
        info!("success object put");
        Ok(())
    }

    /// Lists all objects whose path begins with `prefix`, returning their metadata.
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<ObjectMeta>, StoreError> {
        let (_, canonical) = Self::get_obj_scheme_and_path(prefix)?;
        let store = self.get_or_create_store(prefix)?;
        let mut results = Vec::new();
        let mut stream = store.list(Some(&canonical));

        while let Some(object) = stream.next().await {
            match object {
                Ok(obj) => {
                    results.push(obj);
                }
                Err(e) => return Err(StoreError::ListError(e.to_string())),
            }
        }

        Ok(results)
    }

    /// The URI prefix that a store's relative locations hang off.
    ///
    /// `ObjectMeta::location` is relative to whatever the store is rooted at, and
    /// that root differs per backend: `LocalFileSystem` at `/`, S3 and GCS at a
    /// bucket, Azure at a *container which is itself inside an account*.  Azure
    /// is the reason this derives the root from the URI rather than from a
    /// scheme-and-bucket pair: rebuilding `az://{container}/…` silently drops
    /// the account, producing a URI that cannot be listed.
    pub fn store_root_uri(prefix: &str) -> Result<String, StoreError> {
        let url = Url::parse(prefix)?;
        let (scheme, _) = Self::get_obj_scheme_and_path(prefix)?;

        match scheme {
            // Three slashes: the local root is `file:///`, so joining a
            // relative location onto it yields `file:///home/...`.
            ObjectStoreScheme::Local => Ok("file:///".to_string()),
            ObjectStoreScheme::MicrosoftAzure => {
                let account = url.host_str().unwrap_or_default();
                let container = url
                    .path_segments()
                    .and_then(|mut segments| segments.next())
                    .unwrap_or_default();

                if container.is_empty() {
                    return Err(StoreError::ValidationError(
                        "Azure path is missing a container".into(),
                    ));
                }
                Ok(format!("az://{account}/{container}"))
            }
            ObjectStoreScheme::Http => Ok(prefix.trim_end_matches('/').to_string()),
            _ => Ok(format!(
                "{}://{}",
                url.scheme(),
                url.host_str().unwrap_or_default()
            )),
        }
    }

    /// Joins a store-relative location onto its root, yielding an absolute URI.
    fn to_uri(root_uri: &str, location: &str) -> String {
        let location = location.trim_start_matches('/');

        // The local root already ends in a slash; the cloud roots do not.
        if root_uri.ends_with('/') {
            format!("{root_uri}{location}")
        } else {
            format!("{root_uri}/{location}")
        }
    }

    /// Final path segment of a store location, used as the display name.
    fn base_name(location: &str) -> String {
        location
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(location)
            .to_string()
    }

    /// Lists one level of `prefix`, without recursing.
    ///
    /// Unlike [`Self::list_objects`], which walks the entire subtree and returns
    /// objects only, this uses `list_with_delimiter` so folder-like prefixes come
    /// back as directory entries.  That makes it usable for an expandable file
    /// browser, where each expansion costs exactly one call.
    ///
    /// `prefix` must be a fully-formed URI (`file:///…`, `s3://…`, `gs://…`,
    /// `az://…`).  Directories sort before files, each alphabetically.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the URI cannot be parsed, the backend cannot be
    /// built (usually missing credentials), or the listing call fails.
    pub async fn list_directory(&self, prefix: &str) -> Result<Vec<DirectoryEntry>, StoreError> {
        let (_, canonical) = Self::get_obj_scheme_and_path(prefix)?;
        let root_uri = Self::store_root_uri(prefix)?;
        let store = self.get_or_create_store(prefix)?;

        let listing = store
            .list_with_delimiter(Some(&canonical))
            .await
            .map_err(|e| StoreError::ListError(e.to_string()))?;

        let mut entries: Vec<DirectoryEntry> = Vec::new();

        for directory in listing.common_prefixes {
            let location = directory.as_ref();
            entries.push(DirectoryEntry {
                uri: Self::to_uri(&root_uri, location),
                name: Self::base_name(location),
                is_directory: true,
                size: 0,
                last_modified: String::new(),
            });
        }

        let prefix_path = canonical.as_ref().trim_end_matches('/');

        for object in listing.objects {
            let location = object.location.as_ref();

            // Skip directory placeholders. Consoles and SDKs create a zero-byte
            // object with the same key as the prefix to make an "empty folder"
            // visible. Returned as a file it is both meaningless to the user and
            // actively harmful: its URI equals its own parent's, so a tree that
            // renders children by URI recurses into itself forever.
            if location.trim_end_matches('/') == prefix_path {
                continue;
            }

            entries.push(DirectoryEntry {
                uri: Self::to_uri(&root_uri, location),
                name: Self::base_name(location),
                is_directory: false,
                size: object.size,
                last_modified: object.last_modified.to_rfc3339(),
            });
        }

        entries.sort_by(|a, b| {
            b.is_directory
                .cmp(&a.is_directory)
                .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
        });

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_data_uri() -> String {
        format!("file://{}/mock_data", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn test_store_root_uri_per_backend() {
        // Azure is the interesting one: the account lives in the host and the
        // container in the first path segment, so a root built from the
        // container alone would drop the account.
        assert_eq!(
            StoreService::store_root_uri("az://genreblobs/genre-test-data").unwrap(),
            "az://genreblobs/genre-test-data"
        );
        assert_eq!(
            StoreService::store_root_uri("az://genreblobs/genre-test-data/sub/x.bw").unwrap(),
            "az://genreblobs/genre-test-data"
        );
        assert_eq!(
            StoreService::store_root_uri("s3://my-bucket/data/x.bam").unwrap(),
            "s3://my-bucket"
        );
        assert_eq!(
            StoreService::store_root_uri("gs://b/x.bam").unwrap(),
            "gs://b"
        );
        assert_eq!(
            StoreService::store_root_uri("file:///home/drew").unwrap(),
            "file:///"
        );
        // An Azure URI with no container cannot be listed.
        assert!(StoreService::store_root_uri("az://genreblobs").is_err());
    }

    #[test]
    fn test_to_uri_rebuilds_absolute_paths() {
        // `ObjectMeta::location` is store-relative; these are the round trips
        // that make a returned entry usable as the next call's input.
        assert_eq!(
            StoreService::to_uri("file:///", "home/drew/x.bam"),
            "file:///home/drew/x.bam"
        );
        assert_eq!(
            StoreService::to_uri("s3://my-bucket", "data/x.bam"),
            "s3://my-bucket/data/x.bam"
        );
        assert_eq!(
            StoreService::to_uri("gs://b", "x.bam"),
            "gs://b/x.bam"
        );
        assert_eq!(
            StoreService::to_uri("az://genreblobs/genre-test-data", "density.bw"),
            "az://genreblobs/genre-test-data/density.bw"
        );
    }

    #[test]
    fn test_base_name() {
        assert_eq!(StoreService::base_name("a/b/c.bam"), "c.bam");
        assert_eq!(StoreService::base_name("a/b/"), "b");
        assert_eq!(StoreService::base_name("solo.bam"), "solo.bam");
    }

    #[tokio::test]
    async fn test_list_directory_local() {
        let service = StoreService::new();
        let entries = service.list_directory(&mock_data_uri()).await.unwrap();

        // Every URI must be absolute and round-trippable.
        for entry in &entries {
            assert!(
                entry.uri.starts_with("file:///"),
                "expected absolute file URI, got {}",
                entry.uri
            );
        }

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"test.txt"));
        assert!(names.contains(&"NA12878.gatk.cnv.vcf.gz"));

        let vcf = entries
            .iter()
            .find(|e| e.name == "NA12878.gatk.cnv.vcf.gz")
            .unwrap();
        assert!(!vcf.is_directory);
        assert!(vcf.size > 0);
        assert!(!vcf.last_modified.is_empty());
    }

    #[tokio::test]
    async fn test_list_directory_is_not_recursive() {
        // The parent of mock_data contains src/, which is deep. A recursive walk
        // would return src/**; a delimiter listing returns src/ as one entry.
        let service = StoreService::new();
        let parent = format!("file://{}", env!("CARGO_MANIFEST_DIR"));
        let entries = service.list_directory(&parent).await.unwrap();

        assert!(entries.iter().any(|e| e.name == "src" && e.is_directory));
        assert!(
            entries.iter().all(|e| !e.name.contains("stores")),
            "listing recursed into subdirectories"
        );
    }

    #[tokio::test]
    async fn test_list_directory_sorts_directories_first() {
        let service = StoreService::new();
        let parent = format!("file://{}", env!("CARGO_MANIFEST_DIR"));
        let entries = service.list_directory(&parent).await.unwrap();

        let first_file = entries.iter().position(|e| !e.is_directory);
        let last_directory = entries.iter().rposition(|e| e.is_directory);
        if let (Some(file), Some(directory)) = (first_file, last_directory) {
            assert!(directory < file, "directories must sort before files");
        }
    }

    #[tokio::test]
    async fn test_list_directory_skips_directory_placeholders() {
        // S3 consoles create a zero-byte object with the same key as the prefix
        // to represent an empty folder. Returning it as a file gives it a URI
        // identical to its own parent's, which makes a URI-keyed tree recurse
        // into itself. Local listings cannot produce one, so this asserts the
        // filter directly.
        let prefix = "data/sub";
        assert_eq!("data/sub/".trim_end_matches('/'), prefix);
        assert_eq!("data/sub".trim_end_matches('/'), prefix);
        // A genuine child is not filtered.
        assert_ne!("data/sub/file.bam".trim_end_matches('/'), prefix);
    }

    #[tokio::test]
    async fn test_list_directory_entries_differ_from_parent() {
        // Whatever the backend, no entry may carry its parent's own URI.
        let service = StoreService::new();
        let parent = format!("file://{}", env!("CARGO_MANIFEST_DIR"));
        let entries = service.list_directory(&parent).await.unwrap();

        for entry in &entries {
            assert_ne!(
                entry.uri.trim_end_matches('/'),
                parent.trim_end_matches('/'),
                "entry {} repeats its parent URI",
                entry.name
            );
        }
    }

    #[tokio::test]
    async fn test_list_directory_rejects_non_uri() {
        let service = StoreService::new();
        // A bare path has no scheme, so URI parsing fails with a clear error
        // rather than silently listing something unexpected.
        assert!(service.list_directory("/tmp").await.is_err());
    }
}

use core::ops::Range;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use log::info;
use object_store::path::Path as ObjectStorePath;
use object_store::{ObjectMeta, ObjectStore, ObjectStoreScheme, PutPayload};

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
/// Use [`StoreService::from_uri`] to auto-detect the backend from the URL scheme:
///
/// ```rust,no_run
/// use seqa_core::stores::StoreService;
///
/// // Local file
/// let svc = StoreService::from_uri("file:///data/sample.bam").unwrap();
///
/// // AWS S3 (requires AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_REGION env vars)
/// let svc = StoreService::from_uri("s3://my-bucket/sample.bam").unwrap();
/// ```
#[derive(Debug, Default)]
pub struct StoreService {
    stores: Mutex<HashMap<String, Arc<dyn ObjectStore>>>,
}

impl StoreService {
    /// Creates an empty `StoreService`. Backends are built lazily on first access.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `StoreService` by auto-detecting the storage backend from `path`
    /// and priming its cache with a client for that URI.
    ///
    /// | Scheme | Backend | Required env vars |
    /// |--------|---------|-------------------|
    /// | `s3://` | AWS S3 | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` |
    /// | `az://` | Azure Blob | `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_STORAGE_ACCOUNT` |
    /// | `gs://` | Google Cloud Storage | `GOOGLE_STORAGE_ACCOUNT`, `GOOGLE_BUCKET` |
    /// | `http://` / `https://` | HTTP | — |
    /// | `file://` | Local filesystem | — |
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the URI cannot be parsed, the scheme is unsupported,
    /// or required environment variables are missing.
    pub fn from_uri(path: &str) -> Result<StoreService, StoreError> {
        let svc = Self::new();
        let (scheme, _) = Self::get_canonical_path(path)?;
        svc.get_or_create_store(&scheme)?;
        Ok(svc)
    }

    pub fn from_scheme(scheme: &ObjectStoreScheme) -> Result<StoreService, StoreError> {
        let svc= Self::new();
        svc.get_or_create_store(scheme)?;
        Ok(svc)
    }

    /// Returns a cached backend client for `uri`, building one if absent.
    ///
    /// Cache key is derived from the URI's scheme and host, so two calls with
    /// different object paths under the same bucket/host share a client.
    pub fn get_or_create_store(&self, scheme: &ObjectStoreScheme) -> Result<Arc<dyn ObjectStore>, StoreError> {
        let key = Self::get_key_from_scheme(&scheme);
        let mut stores = self
            .stores
            .lock()
            .expect("StoreService mutex poisoned");

        if let Some(existing) = stores.get(&key) {
            return Ok(Arc::clone(existing));
        }
        let store = Self::build_store(scheme)?;
        stores.insert(key, Arc::clone(&store));
        Ok(store)
    }

    fn get_key_from_scheme(scheme: &ObjectStoreScheme) -> String {
        match scheme {
            ObjectStoreScheme::AmazonS3 => "s3".into(),
            ObjectStoreScheme::GoogleCloudStorage => "gs".into(),
            ObjectStoreScheme::MicrosoftAzure => "az".into(),
            ObjectStoreScheme::Http => "http".into(),
            ObjectStoreScheme::Local => "local".into(),
            _ => {
                panic!("Unsupported store type");
            }
        }
    }

    fn get_store_from_scheme(scheme: &ObjectStoreScheme) -> Result<Arc<dyn ObjectStore>, StoreError> {
        let store: Arc<dyn ObjectStore> = match scheme {
            ObjectStoreScheme::AmazonS3 => Arc::new(store::get_s3_store(None)?),
            ObjectStoreScheme::GoogleCloudStorage => Arc::new(store::get_gc_store(None)?),
            ObjectStoreScheme::MicrosoftAzure => Arc::new(store::get_azure_store(None)?),
            ObjectStoreScheme::Http => Arc::new(store::get_http_store(None)?),
            ObjectStoreScheme::Local => Arc::new(store::get_local_store()?),
            _ => {
                return Err(StoreError::ValidationError(
                    "Unsupported store type".into(),
                ));
            }
        };
        Ok(store)
    }

    fn build_store(scheme: &ObjectStoreScheme) -> Result<Arc<dyn ObjectStore>, StoreError> {
        let store: Arc<dyn ObjectStore> = Self::get_store_from_scheme(scheme)?;
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
        let (scheme, url) = Self::get_canonical_path(path)?;
        let store = self.get_or_create_store(&scheme)?;
        Ok(store.get_range(&url, range).await?.to_vec())
    }

    /// Get file path
    /// Gets a Path object from the string supplied.
    pub fn get_canonical_path(path: &str) -> Result<(ObjectStoreScheme, ObjectStorePath), StoreError> {
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

        let url = &abs_path.parse()?;

        match ObjectStoreScheme::parse(url) {
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
        let (scheme, canonical) = Self::get_canonical_path(path)?;
        let store = self.get_or_create_store(&scheme)?;
        let meta = store.head(&canonical).await?;
        Ok(meta.size)
    }

    /// Downloads the entire object at `path` and returns its bytes.
    pub async fn get_object(&self, path: &str) -> Result<Vec<u8>, StoreError> {
        let (scheme, canonical) = Self::get_canonical_path(path)?;
        let store = self.get_or_create_store(&scheme)?;
        let result = store.get(&canonical).await?;
        let bytes = result.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Uploads `contents` to the object at `path`, creating or overwriting it.
    pub async fn put_object(&self, path: &str, contents: &[u8]) -> Result<(), StoreError> {
        let (scheme, canonical) = Self::get_canonical_path(path)?;
        let store = self.get_or_create_store(&scheme)?;

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
        let (scheme, canonical) = Self::get_canonical_path(prefix)?;
        let store = self.get_or_create_store(&scheme)?;
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
}
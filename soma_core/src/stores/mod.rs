use core::ops::Range;
use futures::StreamExt;
use object_store::path::Path as ObjectStorePath;
use object_store::{ObjectMeta, ObjectStore, ObjectStoreScheme, PutPayload};

pub mod error;
pub mod store;

use error::StoreError;

#[derive(Debug)]
pub struct StoreService {
    store: Box<dyn ObjectStore>,
}

impl StoreService {
    pub fn new(service_type: ObjectStoreScheme) -> Result<Self, StoreError> {
        let store = match service_type {
            ObjectStoreScheme::AmazonS3 => {
                store::get_s3_store(None).map(|s3| Box::new(s3) as Box<dyn ObjectStore>)
            }
            ObjectStoreScheme::GoogleCloudStorage => {
                store::get_gc_store(None).map(|gc| Box::new(gc) as Box<dyn ObjectStore>)
            }
            ObjectStoreScheme::MicrosoftAzure => {
                store::get_azure_store(None).map(|az| Box::new(az) as Box<dyn ObjectStore>)
            }
            ObjectStoreScheme::Local => {
                store::get_local_store().map(|local| Box::new(local) as Box<dyn ObjectStore>)
            }
            _ => return Err(StoreError::ValidationError("Unsupported store type. Either we dont support your store or you tried to initialize a HTTP store without the URL".into())),
        };
        Ok(StoreService { store: store? })
    }

    pub fn from_uri(path: &str) -> Result<StoreService, StoreError> {
        let url = path.parse()?;

        let (scheme, _path) = match ObjectStoreScheme::parse(&url) {
            Ok((scheme, path)) => (scheme, path),
            Err(e) => {
                return Err(StoreError::ObjectStoreUriParseError(e.to_string()));
            }
        };

        let service_store = match scheme {
            ObjectStoreScheme::AmazonS3 => {
                store::get_s3_store(Some(path)).map(|s3| Box::new(s3) as Box<dyn ObjectStore>)?
            }
            ObjectStoreScheme::GoogleCloudStorage => {
                store::get_gc_store(None).map(|gc| Box::new(gc) as Box<dyn ObjectStore>)?
            }
            ObjectStoreScheme::MicrosoftAzure => {
                store::get_azure_store(None).map(|az| Box::new(az) as Box<dyn ObjectStore>)?
            }
            ObjectStoreScheme::Http => {
                store::get_http_store(path).map(|http| Box::new(http) as Box<dyn ObjectStore>)?
            }
            ObjectStoreScheme::Local => {
                store::get_local_store().map(|local| Box::new(local) as Box<dyn ObjectStore>)?
            }
            _ => {
                return Err(StoreError::ValidationError("Unsupported store type".into()));
            }
        };

        Ok(StoreService {
            store: service_store,
        })
    }

    pub fn get_store(&self) -> &Box<dyn ObjectStore> {
        &self.store
    }

    // Get range
    pub async fn get_range(&self, path: &str, range: Range<u64>) -> Result<Vec<u8>, StoreError> {
        let path = match Self::get_canonical_path(path) {
            Ok(path) => path,
            Err(e) => {
                return Err(StoreError::ValidationError(format!(
                    "validation error: {}",
                    e
                )));
            }
        };
        Ok(self
            .get_store()
            .get_range(&object_store::path::Path::from(path), range)
            .await?
            .to_vec())
    }

    /// Get file path
    /// Gets Path object from the string supplied.
    pub fn get_canonical_path(path: &str) -> Result<ObjectStorePath, StoreError> {
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
                    ObjectStoreScheme::MicrosoftAzure => { return Ok(path); }
                    ObjectStoreScheme::AmazonS3 => { return Ok(path); }
                    ObjectStoreScheme::GoogleCloudStorage => { return Ok(path); }
                    ObjectStoreScheme::Http => { return Ok(path); }
                    ObjectStoreScheme::Local => { return Ok(path); }
                    _ => {
                        return Err(StoreError::ValidationError(
                            "Unsupported store type".into(),
                        ))
                    }
                }
            },
            Err(e) => {
                return Err(StoreError::ObjectStoreUriParseError(e.to_string()));
            }
        };
    }

    pub async fn get_file_size(&self, path: &str) -> Result<u64, StoreError> {
        let path = Self::get_canonical_path(path)?;
        let meta = self.get_store().head(&ObjectStorePath::from(path)).await?;
        Ok(meta.size)
    }

    // Get object
    pub async fn get_object(&self, path: &str) -> Result<Vec<u8>, StoreError> {
        let canonical_path = Self::get_canonical_path(path)?;
        let result = self.get_store().get(&canonical_path).await?;
        let bytes = result.bytes().await?;
        Ok(bytes.to_vec())
    }

    // Put object
    pub async fn put_object(&self, path: &str, contents: &[u8]) -> Result<(), StoreError> {
        let canonical_path = Self::get_canonical_path(path)?;
        let store = self.get_store();

        // Create a payload from the file content
        let payload = PutPayload::from(contents.to_vec());

        // Upload the object
        store
            .put(&canonical_path, payload)
            .await
            .map_err(|e| StoreError::PutError(e.to_string()))?;
        println!("success object put");
        Ok(())
    }

    // List objects
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<ObjectMeta>, StoreError> {
        let prefix = match Self::get_canonical_path(prefix) {
            Ok(path) => path,
            Err(e) => {
                return Err(StoreError::ValidationError(format!(
                    "validation error: {}",
                    e
                )));
            }
        };
        let mut results = Vec::new();
        let mut stream = self.get_store().list(Some(&ObjectStorePath::from(prefix)));

        while let Some(object) = stream.next().await {
            match object {
                Ok(obj) => {
                    // obj.location is already a Path, convert it to string
                    results.push(obj);
                }
                Err(e) => return Err(StoreError::ListError(e.to_string())),
            }
        }

        Ok(results)
    }
}


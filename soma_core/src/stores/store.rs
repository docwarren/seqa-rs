use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::azure::{MicrosoftAzure, MicrosoftAzureBuilder};
use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};
use object_store::http::{HttpBuilder, HttpStore};
use object_store::local::LocalFileSystem;

use url::Url;

use crate::stores::error::StoreError;

/// Extracts the bucket name from an S3 URL.
/// For s3:// URLs, the bucket is the host (e.g. s3://bucket/key -> "bucket").
/// For https:// S3 URLs, the bucket is the first path segment
/// (e.g. https://s3.region.amazonaws.com/bucket/key -> "bucket").
pub fn get_s3_bucket_from_url(s3_url: &str) -> Option<String> {
    let url = Url::parse(s3_url).ok()?;
    match url.scheme() {
        "s3" | "s3a" => url.host_str().map(|h| h.to_string()),
        "https" | "http" => url.path_segments()?.next().map(|s| s.to_string()),
        _ => None,
    }
}

pub fn get_s3_store(url: Option<&str>) -> Result<AmazonS3, StoreError> {

    let builder = match url {
        Some(url) => AmazonS3Builder::from_env().with_url(url),
        None => AmazonS3Builder::from_env(),
    };
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_s3_bucket_from_s3_url() {
        assert_eq!(
            get_s3_bucket_from_url("s3://com.gmail.docarw/test.txt"),
            Some("com.gmail.docarw".to_string())
        );
    }

    #[test]
    fn test_get_s3_bucket_from_https_url() {
        assert_eq!(
            get_s3_bucket_from_url("https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test.txt"),
            Some("com.gmail.docarw".to_string())
        );
    }

    #[test]
    fn test_get_s3_bucket_from_invalid_url() {
        assert_eq!(get_s3_bucket_from_url("not a url"), None);
    }
}

pub fn get_gc_store(bucket: Option<String>) -> Result<GoogleCloudStorage, StoreError> {
    let bucket_name = match bucket {
        Some(bucket_name) => bucket_name,
        _ => std::env::var("GOOGLE_BUCKET").unwrap_or_default()
    };

    let mut builder = GoogleCloudStorageBuilder::new()
        .with_bucket_name(bucket_name);

    if let Ok(key) = std::env::var("GOOGLE_SERVICE_ACCOUNT_KEY") {
        builder = builder.with_service_account_key(key);
    } else if let Ok(path) = std::env::var("GOOGLE_SERVICE_ACCOUNT") {
        builder = builder.with_service_account_path(path);
    }

    Ok(builder.build()?)
}

pub fn get_azure_store(bucket: Option<String>) -> Result<MicrosoftAzure, StoreError> {

    let account_id = std::env::var("AZURE_STORAGE_ACCOUNT").unwrap_or_default();
    let container_name = match bucket {
        Some(bucket_name) => bucket_name,
        _ => std::env::var("AZURE_STORAGE_CONTAINER").unwrap_or_default()
    };
    let key = std::env::var("AZURE_STORAGE_ACCESS_KEY").unwrap_or_default();
    
    Ok(MicrosoftAzureBuilder::new()
        .with_account(account_id)
        .with_access_key(key)
        .with_container_name(container_name)
        .build()?)
}

pub fn get_http_store(path: &str) -> Result<HttpStore, StoreError> {
    Ok(HttpBuilder::new().with_url(path).build()?)
}

pub fn get_local_store() -> Result<LocalFileSystem, StoreError> {
    Ok(LocalFileSystem::new())
}

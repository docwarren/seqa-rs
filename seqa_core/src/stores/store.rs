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

use crate::stores::StoreError;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::azure::{MicrosoftAzure, MicrosoftAzureBuilder};
use object_store::gcp::{GoogleCloudStorage, GoogleCloudStorageBuilder};
use object_store::http::{HttpBuilder, HttpStore};
use object_store::local::LocalFileSystem;

/// Builds an S3 store for `bucket`, with credentials and region from the `AWS_*`
/// environment variables.
pub fn get_s3_store_from_bucket(bucket: &str) -> Result<AmazonS3, StoreError> {
    match AmazonS3Builder::from_env()
        .with_bucket_name(bucket)
        .build() {
        Ok(store) => Ok(store),
        Err(e) => {
            log::error!("{}", e);
            Err(StoreError::StoreNotInitialized(format!("{}", e)))
        },
    }
}

// Google Cloud
pub fn get_gc_store_from_bucket(bucket: &str) -> Result<GoogleCloudStorage, StoreError> {
    match GoogleCloudStorageBuilder::from_env()
        .with_bucket_name(bucket)
        .build() {
        Ok(store) => Ok(store),
        Err(e) => {
            log::error!("{}", e);
            Err(StoreError::StoreNotInitialized(format!("{}", e)))
        },
    }
}

// Azure Blob
pub fn get_azure_store_from_container(container_name: &str) -> Result<MicrosoftAzure, StoreError> {
    match MicrosoftAzureBuilder::from_env()
        .with_container_name(container_name)
        .build() {
        Ok(store) => Ok(store),
        Err(e) => {
            eprintln!("{}", e);
            Err(StoreError::StoreNotInitialized(format!("{}", e)))
        },
    }
}

pub fn get_http_store(path: &str) -> Result<HttpStore, StoreError> {
    Ok(HttpBuilder::new().with_url(path).build()?)
}

pub fn get_local_store() -> Result<LocalFileSystem, StoreError> {
    Ok(LocalFileSystem::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    // These build object stores via *Builder::from_env() and need cloud credentials.
    #[test]
    fn test_get_s3_store_from_bucket() {
        let bucket = "com.gmail.docarw";
        assert!(get_s3_store_from_bucket(bucket).is_ok());
    }

    #[test]
    fn test_get_gc_store_from_bucket() {
        let bucket = "genre_test_bucket";
        assert!(get_gc_store_from_bucket(bucket).is_ok());
    }

    #[test]
    fn test_az_store_from_container_name() {
        let container_name = "genreblobs/genre-test-data";
        let store = get_azure_store_from_container(container_name);
        assert!(store.is_ok());
    }
}

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

use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use log::{debug, error};
use crate::stores::{StoreService, error::StoreError};

/// Determines the local cache path for an index file.
///
/// For remote files (s3://, az://, gs://, http://, https://), returns `./<key>` in the
/// current working directory, where the key is the whole remote path with every character
/// outside `[A-Za-z0-9]` replaced by `_`.
/// For local files, returns the path as-is (stripping the `file://` scheme if present).
///
/// # Arguments
/// * `index_path` - Path to the index file (could be remote)
///
/// # Returns
/// * Local path where the index file should be cached or exists
fn get_local_index_path(index_path: &str) -> PathBuf {
    let is_remote = index_path.starts_with("s3://")
        || index_path.starts_with("az://")
        || index_path.starts_with("gs://")
        || index_path.starts_with("http://")
        || index_path.starts_with("https://");

    if is_remote {
        let key = index_cache_key(index_path);
        PathBuf::from(format!("./{}", key))
    } else {
        let fs_path = index_path.strip_prefix("file://").unwrap_or(index_path);
        PathBuf::from(fs_path)
    }
}

/// Builds a cache file name from a remote index path by replacing every character
/// outside `[A-Za-z0-9]` with `_`.
fn index_cache_key(index_path: &str) -> String {
    index_path
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() { character } else { '_' })
        .collect()
}

/// Checks if an index file exists locally, and if not, downloads it from the remote location.
///
/// # Arguments
/// * `index_path` - The original index path (could be remote)
/// * `no_cache` - When true, skip reading from and writing to the local index cache
///
/// # Returns
/// * Result containing the bytes of the index file
pub(crate) async fn get_or_download_index(
    store_service: &StoreService,
    index_path: &str,
    no_cache: bool,
) -> Result<Vec<u8>, StoreError> {
    let local_path = get_local_index_path(index_path);

    if !no_cache && local_path.exists() {
        return fs::read(&local_path).await.map_err(|e| StoreError::CacheReadError {
            key: index_path.to_string(),
            source: e
        });
    }

    let bytes = store_service.get_object(index_path).await?;

    if !no_cache
        && let Err(e) = cache_index_locally(&local_path, &bytes).await {
            error!("Warning: Failed to cache index locally at {}: {}",
                local_path.display(), e);
        }

    Ok(bytes)
}

/// Writes index bytes to a local cache file.
///
/// # Arguments
/// * `local_path` - Path where to cache the index
/// * `bytes` - Index file contents
///
/// # Returns
/// * Result indicating success or failure
async fn cache_index_locally(local_path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let mut file = fs::File::create(local_path).await?;
    file.write_all(bytes).await?;
    file.flush().await?;

    Ok(())
}

/// Deletes a locally cached index file.
///
/// This is the inverse of `get_or_download_index` — it removes the file that would have
/// been written to the local cache. Safe to call even if the file does not exist.
///
/// # Arguments
/// * `index_path` - Path to the index file (same value passed to `get_or_download_index`)
fn delete_local_index(index_path: &str) {
    let local_path = get_local_index_path(index_path);
    if local_path.exists()
        && let Err(e) = std::fs::remove_file(&local_path) {
            debug!("Warning: Failed to delete cached index {}: {}", local_path.display(), e);
        }
}

/// Deletes a locally cached index file when it goes out of scope.
///
/// Hold one for the length of a test that downloads an index, so the cached file is
/// removed whether the test passes or panics:
///
/// ```rust
/// use seqa_core::indexes::index_cache::IndexGuard;
///
/// let _guard = IndexGuard::new("s3://my-bucket/sample.bam.bai");
/// ```
pub struct IndexGuard(String);

impl IndexGuard {
    /// Creates a guard for `index_path` (the same value passed to `delete_local_index`).
    pub fn new(index_path: &str) -> Self {
        IndexGuard(index_path.to_string())
    }
}

impl Drop for IndexGuard {
    fn drop(&mut self) {
        delete_local_index(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_local_index_path_remote_s3() {
        let path = get_local_index_path("s3://bucket/path/to/file.bam.bai");
        assert_eq!(path, PathBuf::from("./s3___bucket_path_to_file_bam_bai"));
    }

    #[test]
    fn test_get_local_index_path_remote_https() {
        let path = get_local_index_path("https://example.com/data/sample.vcf.gz.tbi");
        assert_eq!(path, PathBuf::from("./https___example_com_data_sample_vcf_gz_tbi"));
    }

    #[test]
    fn test_get_local_index_path_remote_azure() {
        let path = get_local_index_path("az://container/folder/file.bam.bai");
        assert_eq!(path, PathBuf::from("./az___container_folder_file_bam_bai"));
    }

    #[test]
    fn test_get_local_index_path_remote_gcs() {
        let path = get_local_index_path("gs://bucket/data/file.bam.bai");
        assert_eq!(path, PathBuf::from("./gs___bucket_data_file_bam_bai"));
    }

    #[test]
    fn test_get_local_index_path_local() {
        let path = get_local_index_path("/local/path/to/file.bam.bai");
        assert_eq!(path, PathBuf::from("/local/path/to/file.bam.bai"));
    }

    #[test]
    fn test_get_local_index_path_local_relative() {
        let path = get_local_index_path("./data/file.vcf.gz.tbi");
        assert_eq!(path, PathBuf::from("./data/file.vcf.gz.tbi"));
    }

    #[test]
    fn test_get_local_index_path_file_scheme() {
        let path = get_local_index_path("file:///media/user/data/file.bam.bai");
        assert_eq!(path, PathBuf::from("/media/user/data/file.bam.bai"));
    }

    #[test]
    fn test_get_local_index_path_preserves_case() {
        let sample = get_local_index_path("s3://b/Sample.bai");
        let xample = get_local_index_path("s3://b/Xample.bai");
        assert_eq!(sample, PathBuf::from("./s3___b_Sample_bai"));
        assert_eq!(xample, PathBuf::from("./s3___b_Xample_bai"));
    }

    #[test]
    fn test_get_local_index_path_same_name_different_buckets() {
        let first = get_local_index_path("s3://bucket-a/data/sample.bam.bai");
        let second = get_local_index_path("s3://bucket-b/data/sample.bam.bai");
        assert_ne!(first, second);
    }

    #[test]
    fn test_get_local_index_path_same_path_different_schemes() {
        let s3 = get_local_index_path("s3://data/sample.bam.bai");
        let gcs = get_local_index_path("gs://data/sample.bam.bai");
        assert_ne!(s3, gcs);
    }

    #[test]
    fn test_get_local_index_path_host_with_port() {
        let path = get_local_index_path("https://example.com:8080/data/x.tbi");
        assert_eq!(path, PathBuf::from("./https___example_com_8080_data_x_tbi"));
    }

    #[test]
    fn test_get_local_index_path_parent_segments_stay_in_working_directory() {
        let path = get_local_index_path("s3://bucket/../../etc/x.bai");
        let key = path.strip_prefix("./").unwrap().to_str().unwrap();
        assert!(!key.contains('/'));
        assert!(!key.contains('.'));
        assert_eq!(key, "s3___bucket_______etc_x_bai");
    }

    #[tokio::test]
    async fn no_cache_does_not_write_index_file() {
        use crate::api::search::search_features;
        use crate::api::search_options::SearchOptions;
        use crate::stores::StoreService;

        let no_cache_file = "s3://com.gmail.docarw/test_data/test.bedgraph.gz";
        let no_cache_index = "s3://com.gmail.docarw/test_data/test.bedgraph.gz.tbi";
        let _guard = IndexGuard::new(no_cache_index);

        let cache_path = get_local_index_path(no_cache_index);
        assert!(
            !cache_path.exists(),
            "Cache file should not exist before search"
        );

        let options = SearchOptions::new(no_cache_file, &vec!["chr1:1-100000000".to_string()])
            .unwrap()
            .set_index_path(no_cache_index)
            .set_output_format("bedgraph")
            .set_include_header(false)
            .set_no_cache(true);

        let store_service = StoreService::new();
        let result = search_features(&store_service, &options)
            .await
            .expect("Failed to search BEDGRAPH");
        assert!(!result.lines[0].is_empty(), "Search should return results");
        assert!(
            !cache_path.exists(),
            "Cache file should not exist after search with no_cache=true"
        );
    }
}

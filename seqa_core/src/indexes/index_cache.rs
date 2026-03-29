use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use log::{debug, error};
use crate::stores::{StoreService, error::StoreError};

/// Determines the local cache path for an index file.
///
/// For remote files (s3://, az://, gs://, https://), extracts the filename and
/// uses it as the local cache name in the current working directory.
/// For local files, returns the path as-is (stripping the `file://` scheme if present).
///
/// # Arguments
/// * `index_path` - Path to the index file (could be remote)
///
/// # Returns
/// * Local path where the index file should be cached or exists
pub fn get_local_index_path(index_path: &str) -> PathBuf {
    let is_remote = index_path.starts_with("s3://")
        || index_path.starts_with("az://")
        || index_path.starts_with("gs://")
        || index_path.starts_with("http://")
        || index_path.starts_with("https://");

    if is_remote {
        let filename = index_path
            .split('/')
            .last()
            .unwrap_or("index");

        PathBuf::from(format!("./{}", filename))
    } else {
        let fs_path = index_path.strip_prefix("file://").unwrap_or(index_path);
        PathBuf::from(fs_path)
    }
}

/// Checks if an index file exists locally, and if not, downloads it from the remote location.
///
/// # Arguments
/// * `index_path` - The original index path (could be remote)
/// * `no_cache` - When true, skip reading from and writing to the local index cache
///
/// # Returns
/// * Result containing the bytes of the index file
pub async fn get_or_download_index(
    index_path: &str,
    no_cache: bool,
) -> Result<Vec<u8>, StoreError> {
    let local_path = get_local_index_path(index_path);

    // Check if index exists locally (skip when no_cache is set)
    if !no_cache && local_path.exists() {
        // Read from local cache
        match fs::read(&local_path).await {
            Ok(bytes) => {
                return Ok(bytes);
            }
            Err(e) => {
                debug!("Warning: Failed to read local index {}: {}. Downloading from remote.",
                    local_path.display(), e);
            }
        }
    }

    // Index doesn't exist locally or failed to read - download from remote
    let store_service = StoreService::from_uri(index_path)?;
    let bytes = store_service.get_object(index_path).await?;

    // Try to cache the downloaded index locally (skip when no_cache is set)
    if !no_cache {
        if let Err(e) = cache_index_locally(&local_path, &bytes).await {
            error!("Warning: Failed to cache index locally at {}: {}",
                local_path.display(), e);
        }
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
    // Create parent directories if they don't exist
    if let Some(parent) = local_path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Write the file
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
pub fn delete_local_index(index_path: &str) {
    let local_path = get_local_index_path(index_path);
    if local_path.exists() {
        if let Err(e) = std::fs::remove_file(&local_path) {
            debug!("Warning: Failed to delete cached index {}: {}", local_path.display(), e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_local_index_path_remote_s3() {
        let path = get_local_index_path("s3://bucket/path/to/file.bam.bai");
        assert_eq!(path, PathBuf::from("./file.bam.bai"));
    }

    #[test]
    fn test_get_local_index_path_remote_https() {
        let path = get_local_index_path("https://example.com/data/sample.vcf.gz.tbi");
        assert_eq!(path, PathBuf::from("./sample.vcf.gz.tbi"));
    }

    #[test]
    fn test_get_local_index_path_remote_azure() {
        let path = get_local_index_path("az://container/folder/file.bam.bai");
        assert_eq!(path, PathBuf::from("./file.bam.bai"));
    }

    #[test]
    fn test_get_local_index_path_remote_gcs() {
        let path = get_local_index_path("gs://bucket/data/file.bam.bai");
        assert_eq!(path, PathBuf::from("./file.bam.bai"));
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
}

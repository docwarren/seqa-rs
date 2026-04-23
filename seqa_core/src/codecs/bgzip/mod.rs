pub mod bgzip_block;

use super::gzip::gzip_decompress;
use bgzip_block::BgZipBlock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BgZipError {
    #[error("Failed to read BGZIP block: {0}")]
    ReadBlockError(String),

    #[error("Failed to decompress BGZIP block: {0}")]
    DecompressBlockError(#[from] std::io::Error),
}

/// Reads BGZIP blocks from a byte vector
/// Returns a vector of block sizes.
pub fn from_bytes(bytes: &Vec<u8>) -> Result<Vec<usize>, BgZipError> {
    let mut i = 0;
    let mut blocks = Vec::with_capacity(bytes.len() / (16 * 1024) + 1);
    while i < bytes.len() {
        let block = BgZipBlock::from_bytes(&bytes, i);
        match block {
            Ok(block) => {
                let size = block.sub_block.bsize as usize + 1;
                i += size;
                blocks.push(size);
            }
            Err(_) => break,
        }
    }
    Ok(blocks)
}

/// Decompresses BGZIP blocks from a byte vector
/// Takes a vector of block sizes and a byte slice containing the compressed data.
pub fn decompress(block_sizes: &[usize], bytes: &[u8]) -> Result<Vec<u8>, BgZipError> {
    let mut i = 0;
    let mut result = Vec::with_capacity(block_sizes.len());
    let mut zip_handles = Vec::with_capacity(block_sizes.len());

    for &size in block_sizes {
        let compressed: Vec<u8> = bytes[i..i + size].to_vec();

        let handle = std::thread::spawn(move || gzip_decompress(&compressed));
        i += size;
        zip_handles.push(handle);
    }

    for handle in zip_handles {
        match handle.join() {
            Ok(Ok(decompressed)) => result.push(decompressed),
            Ok(Err(e)) => return Err(BgZipError::DecompressBlockError(e)),
            Err(_) => {
                return Err(BgZipError::DecompressBlockError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Thread panicked",
                )))
            }
        }
    }
    Ok(result.iter().flatten().copied().collect())
}

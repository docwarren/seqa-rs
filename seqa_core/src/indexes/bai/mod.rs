use std::array::TryFromSliceError;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::indexes::bin::Bin;
use crate::indexes::chunk::{Chunk, ChunkError};
use super::traits::sam_index::SamIndex;
use super::virtual_offset::VirtualOffset;

pub mod chr_idx;

#[derive(Debug, Error)]
pub enum BaiError {
    #[error("Failed to read BAI index file: {0}")]
    ReadError(String),

    #[error("StoreError: {0}")]
    StoreError(#[from] crate::stores::error::StoreError),

    #[error("Parsing Error: {0}")]
    ParsingError(#[from] TryFromSliceError),

    #[error("Error processing Chunk: {0}")]
    ChunkError(#[from] ChunkError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaiIndex {
    pub magic: [u8; 4],
    pub n_ref: u32,
    pub references: Vec<chr_idx::ChrIdx>,
    pub n_no_coor: u64,
    pub first_feature_offset: VirtualOffset, // Used for figuring out the length of the header
}

impl BaiIndex {
    pub fn new() -> Self {
        BaiIndex {
            magic: [0; 4],
            n_ref: 0,
            references: Vec::new(),
            n_no_coor: 0,
            first_feature_offset: VirtualOffset {
                block_offset: 0,
                decompressed_offset: 0,
                virtual_pointer: u64::MAX,
            },
        }
    }

    /// Creates a BaiIndex from a file, optionally skipping the local cache.
    ///
    /// # Arguments
    /// * `idx_path` - Path to the index file (could be remote)
    /// * `no_cache` - When true, skip reading from and writing to the local index cache
    pub async fn from_file(idx_path: &str, no_cache: bool) -> Result<Self, BaiError> {
        let bytes = crate::indexes::index_cache::get_or_download_index(
            idx_path,
            no_cache
        ).await?;
        BaiIndex::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, BaiError> {
        let mut i = 0;
        let mut bai_index = BaiIndex::new();

        bai_index.magic.copy_from_slice(&bytes[i..i + 4]);
        i += 4;
        bai_index.n_ref = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        // Read references
        for _ in 0..bai_index.n_ref {

            let mut chr_idx = chr_idx::ChrIdx::new();

            let n_bin = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
            i += 4;

            // Read bins
            for _ in 0..n_bin {

                let bin_number = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
                i += 4;

                let n_chunk = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
                i += 4;

                let mut chunks = Vec::with_capacity(n_chunk as usize);
                
                // Read chunks for this bin
                for _ in 0..n_chunk {
                    let chunk = Chunk::from_bytes(&bytes[i..i + 16], bin_number)?;
                    i += 16;
                    chunks.push(chunk);

                    // Update first_feature_offset if this chunk's begin_vp is less than the current first_feature_offset
                    if chunk.begin_vp.virtual_pointer < bai_index.first_feature_offset.virtual_pointer {
                        bai_index.first_feature_offset = chunk.begin_vp.clone();
                    }
                }

                chr_idx.bins.insert(bin_number, Bin {
                    bin: bin_number,
                    chunks,
                });
            }

            let n_intv = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
            i += 4;

            // Read intervals
            for _ in 0..n_intv {
                let interval = u64::from_le_bytes(bytes[i..i + 8].try_into()?);
                i += 8;
                chr_idx.intervals.push(interval);
            }
            bai_index.references.push(chr_idx);
        }
        bai_index.n_no_coor = u64::from_le_bytes(bytes[i..i + 8].try_into()?);

        Ok(bai_index)
    }
}

impl SamIndex for BaiIndex {}

impl BaiIndex {
    pub async fn get_first_feature_offset(&self) -> VirtualOffset {
        return self.first_feature_offset;
    }
}
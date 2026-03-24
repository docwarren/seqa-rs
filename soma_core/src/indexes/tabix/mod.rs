use std::collections::HashMap;
use std::string::FromUtf8Error;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use core::array::TryFromSliceError;
use crate::codecs::bgzip;
use crate::indexes::chunk::{Chunk, ChunkError};
use crate::indexes::bin;
use crate::indexes::virtual_offset::VirtualOffset;
use crate::stores::error::StoreError;

use super::bai::chr_idx::ChrIdx;
use super::traits::sam_index::SamIndex;

#[derive(Debug, Error)]
pub enum TabixError {
    #[error("Failed to read Tabix index file: {0}")]
    ReadError(String),

    #[error("Failed to parse Tabix index file: {0}")]
    ParseError(#[from] FromUtf8Error),

    #[error("Failed to decompress Tabix index file: {0}")]
    DecompressError(#[from] bgzip::BgZipError),

    #[error("Store error: {0}")]
    StoreError(#[from] StoreError),

    #[error("Parsing Error: {0}")]
    ParsingError(#[from] TryFromSliceError),

    #[error("Chunk Error: {0}")]
    ChunkError(#[from] ChunkError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tabix {
    pub magic: String,
    pub n_ref: i32,
    pub format: i32,
    pub col_seq: i32,
    pub col_beg: i32,
    pub col_end: i32,
    pub meta: i32,
    pub skip: i32,
    pub l_nm: i32,
    pub names: Vec<String>,
    pub references: Vec<ChrIdx>,
    pub n_no_coor: u64,
    pub first_feature_offset: VirtualOffset
}

impl Tabix {
    pub fn new() -> Self {
        Tabix {
            magic: String::new(),
            n_ref: 0,
            format: 0,
            col_seq: 0,
            col_beg: 0,
            col_end: 0,
            meta: 0,
            skip: 0,
            l_nm: 0,
            names: Vec::new(),
            references: Vec::new(),
            n_no_coor: 0,
            first_feature_offset: VirtualOffset {
                block_offset: 0,
                decompressed_offset: 0,
                virtual_pointer: u64::MAX,
            },
        }
    }

    pub async fn from_compressed_file(idx_path: &str) -> Result<Self, TabixError> {
        let bytes = crate::indexes::index_cache::get_or_download_index(
            idx_path,
            ".tbi",
            idx_path
        ).await?;
        Ok(Tabix::from_compressed_bytes(bytes)?)
    }

    /// Creates a Tabix index from a compressed file with explicit data file path for cache naming.
    ///
    /// # Arguments
    /// * `idx_path` - Path to the index file (could be remote)
    /// * `data_file_path` - Path to the data file (used for cache naming)
    pub async fn from_compressed_file_with_data_path(idx_path: &str, data_file_path: &str) -> Result<Self, TabixError> {
        let bytes = crate::indexes::index_cache::get_or_download_index(
            idx_path,
            ".tbi",
            data_file_path
        ).await?;
        Ok(Tabix::from_compressed_bytes(bytes)?)
    }

    pub fn from_compressed_bytes(bytes: Vec<u8>) -> Result<Self, TabixError> {
        let block_sizes = bgzip::from_bytes(&bytes)?;
        let decompressed = bgzip::decompress(&block_sizes, &bytes)?;
        Ok(Tabix::from_bytes(decompressed)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, TabixError> {
        let mut i = 0;
        let mut tabix = Tabix::new();

        tabix.magic = String::from_utf8(bytes[i..i + 4].to_vec())?;
        i += 4;
        tabix.n_ref = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;
        tabix.format = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;
        tabix.col_seq = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;
        tabix.col_beg = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;
        tabix.col_end = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;
        tabix.meta = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;
        tabix.skip = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;
        tabix.l_nm = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        // Read names
        tabix.names = bytes[i..(i + tabix.l_nm as usize)]
            .iter()
            .map(|&b| b as char)
            .collect::<String>()
            .split('\0')
            .map(|s| s.to_string())
            .collect();

        // Remove the last empty string if it exists
        if let Some(last) = tabix.names.last() {
            if last.is_empty() {
                tabix.names.pop();
            }
        }

        i += tabix.l_nm as usize;

        // Read indices
        for _ in 0..tabix.n_ref {
            // Read TabixIndex
            let mut chr_index = ChrIdx {
                bins: HashMap::new(),
                intervals: Vec::new(),
            };

            let n_bin = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
            i += 4;

            for _ in 0..n_bin {
                // Read TabixBin
                let mut tabix_bin = bin::Bin {
                    bin: 0,
                    chunks: Vec::new(),
                };

                tabix_bin.bin = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
                i += 4;
                let n_chunk = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
                i += 4;

                let mut chunks = Vec::with_capacity(n_chunk as usize);

                for _ in 0..n_chunk {
                    let chunk = Chunk::from_bytes(&bytes[i..i + 16], tabix_bin.bin)?;
                    i += 16; // Each chunk is 16 bytes (8 for begin, 8 for end)
                    chunks.push(chunk);

                    // Update first_feature_offset if this chunk's begin_vp is less than the current first_feature_offset
                    if chunk.begin_vp.virtual_pointer < tabix.first_feature_offset.virtual_pointer {
                        tabix.first_feature_offset = chunk.begin_vp.clone();
                    }
                }

                tabix_bin.chunks = chunks;
                chr_index.bins.insert(tabix_bin.bin, tabix_bin);
            }

            let n_intv = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
            i += 4;

            let mut intervals = Vec::with_capacity(n_intv as usize);

            for _ in 0..n_intv {
                let interval = u64::from_le_bytes(bytes[i..i + 8].try_into()?);
                i += 8;
                intervals.push(interval);
            }

            chr_index.intervals = intervals;

            tabix.references.push(chr_index);
        }

        Ok(tabix)
    }

    pub fn get_chromosome_index_by_name(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }
}

impl SamIndex for Tabix {}

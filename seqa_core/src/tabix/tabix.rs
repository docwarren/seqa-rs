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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;
use crate::codecs::bgzip;
use crate::indexes::bin;
use crate::indexes::chr_idx::ChrIdx;
use crate::indexes::chunk::Chunk;
use crate::indexes::virtual_offset::VirtualOffset;
use crate::stores::StoreService;
use crate::traits::sam_index::SamIndex;

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

impl Default for Tabix {
    fn default() -> Self {
        Self::new()
    }
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

    /// Creates a Tabix index from a compressed file, optionally skipping the local cache.
    ///
    /// # Arguments
    /// * `idx_path` - Path to the index file (could be remote)
    /// * `no_cache` - When true, skip reading from and writing to the local index cache
    pub async fn from_compressed_file(
        store_service: &StoreService,
        idx_path: &str,
        no_cache: bool,
    ) -> Result<Self, SearchError> {
        let bytes = crate::indexes::index_cache::get_or_download_index(
            store_service,
            idx_path,
            no_cache)
            .await
            .map_err(|e| SearchError::ReadError {
                path: idx_path.to_string(),
                source: e
            })?;
        Tabix::from_compressed_bytes(bytes, idx_path)
    }

    pub fn from_compressed_bytes(bytes: Vec<u8>, file_path: &str) -> Result<Self, SearchError> {
        let block_sizes = bgzip::from_bytes(&bytes).map_err(|e| SearchError::CodecError {
            source: e,
            path: file_path.to_string()
        })?;
        let decompressed = bgzip::decompress(&block_sizes, &bytes)
            .map_err(|e| SearchError::CodecError {
                source: e,
                path: file_path.to_string()
            })?;
        Tabix::from_bytes(decompressed).map_err(|e| SearchError::ParseError {
            path: file_path.to_string(),
            source: e
        })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ParsingError> {
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
        if let Some(last) = tabix.names.last() && last.is_empty() {
            tabix.names.pop();
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
                    let chunk = Chunk::from_bytes(&bytes[i..i + 16], tabix_bin.bin)
                        .map_err(|e| ParsingError::ChunkError {
                            source: e,
                        })?;
                    i += 16; // Each chunk is 16 bytes (8 for begin, 8 for end)
                    chunks.push(chunk);

                    // Update first_feature_offset if this chunk's begin_vp is less than the current first_feature_offset
                    if chunk.begin_vp.virtual_pointer < tabix.first_feature_offset.virtual_pointer {
                        tabix.first_feature_offset = chunk.begin_vp;
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
        crate::genome::chromosome_aliases(name)
            .iter()
            .find_map(|alias| self.names.iter().position(|n| n == alias))
    }
}

impl SamIndex for Tabix {}

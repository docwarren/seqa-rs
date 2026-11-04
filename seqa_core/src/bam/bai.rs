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
use crate::indexes::bin::Bin;
use crate::indexes::chr_idx::ChrIdx;
use crate::indexes::chunk::Chunk;
use crate::indexes::virtual_offset::VirtualOffset;
use crate::stores::StoreService;
use crate::traits::sam_index::SamIndex;
use serde::{Deserialize, Serialize};
use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaiIndex {
    pub magic: [u8; 4],
    pub n_ref: u32,
    pub references: Vec<ChrIdx>,
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
    pub async fn from_file(
        store_service: &StoreService,
        idx_path: &str,
        no_cache: bool,
    ) -> Result<Self, SearchError> {
        let bytes = crate::indexes::index_cache::get_or_download_index(
            store_service,
            idx_path,
            no_cache
        ).await.map_err(|e| SearchError::ReadError { path: idx_path.to_string(), source: e })?;
        BaiIndex::from_bytes(bytes).map_err(|e| SearchError::ParseError {
            path: idx_path.to_string(),
            source: e
        })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ParsingError> {
        let mut i = 0;
        let mut bai_index = BaiIndex::new();

        bai_index.magic.copy_from_slice(&bytes[i..i + 4]);
        i += 4;
        bai_index.n_ref = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        // Read references
        for _ in 0..bai_index.n_ref {

            let mut chr_idx = ChrIdx::new();

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
                    let chunk = Chunk::from_bytes(&bytes[i..i + 16], bin_number)
                        .map_err(|e| ParsingError::ChunkError {
                            source: e
                        })?;
                    i += 16;
                    chunks.push(chunk);

                    // Update first_feature_offset if this chunk's begin_vp is less than the current first_feature_offset
                    if chunk.begin_vp.virtual_pointer < bai_index.first_feature_offset.virtual_pointer {
                        bai_index.first_feature_offset = chunk.begin_vp;
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
        self.first_feature_offset
    }
}
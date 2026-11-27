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
pub struct TabixIndex {
    pub(crate) magic: String,
    pub(crate) n_ref: i32,
    pub(crate) format: i32,
    pub(crate) col_seq: i32,
    pub(crate) col_beg: i32,
    pub(crate) col_end: i32,
    pub(crate) meta: i32,
    pub(crate) skip: i32,
    pub(crate) l_nm: i32,
    pub(crate) names: Vec<String>,
    pub(crate) references: Vec<ChrIdx>,
    pub(crate) n_no_coor: u64,
    pub(crate) first_feature_offset: VirtualOffset
}

impl Default for TabixIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl TabixIndex {
    pub fn new() -> Self {
        TabixIndex {
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
        TabixIndex::from_compressed_bytes(bytes, idx_path)
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
        Ok(TabixIndex::from_bytes(decompressed)?)
    }

    /// Parses a decompressed tabix index, returning an error when the bytes are truncated.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ParsingError> {
        let mut i = 0;
        let mut tabix = TabixIndex::new();

        let magic_bytes = read_slice(&bytes, i, 4)?;
        tabix.magic = String::from_utf8(magic_bytes.to_vec())?;
        i += 4;
        tabix.n_ref = read_i32(&bytes, i)?;
        i += 4;
        tabix.format = read_i32(&bytes, i)?;
        i += 4;
        tabix.col_seq = read_i32(&bytes, i)?;
        i += 4;
        tabix.col_beg = read_i32(&bytes, i)?;
        i += 4;
        tabix.col_end = read_i32(&bytes, i)?;
        i += 4;
        tabix.meta = read_i32(&bytes, i)?;
        i += 4;
        tabix.skip = read_i32(&bytes, i)?;
        i += 4;
        tabix.l_nm = read_i32(&bytes, i)?;
        i += 4;

        let name_bytes = read_slice(&bytes, i, tabix.l_nm as usize)?;
        tabix.names = name_bytes
            .iter()
            .map(|&b| b as char)
            .collect::<String>()
            .split('\0')
            .map(|s| s.to_string())
            .collect();

        if let Some(last) = tabix.names.last() && last.is_empty() {
            tabix.names.pop();
        }

        i += tabix.l_nm as usize;

        for _ in 0..tabix.n_ref {
            let mut chr_index = ChrIdx {
                bins: HashMap::new(),
                intervals: Vec::new(),
            };

            let n_bin = read_i32(&bytes, i)?;
            i += 4;

            for _ in 0..n_bin {
                let mut tabix_bin = bin::Bin {
                    bin: 0,
                    chunks: Vec::new(),
                };

                tabix_bin.bin = read_u32(&bytes, i)?;
                i += 4;
                let n_chunk = read_i32(&bytes, i)?;
                i += 4;

                let mut chunks = Vec::with_capacity(n_chunk as usize);

                for _ in 0..n_chunk {
                    let chunk_bytes = read_slice(&bytes, i, 16)?;
                    let chunk = Chunk::from_bytes(chunk_bytes, tabix_bin.bin)
                        .map_err(|e| ParsingError::ChunkError {
                            source: e,
                        })?;
                    i += 16;
                    chunks.push(chunk);

                    if chunk.begin_vp.virtual_pointer < tabix.first_feature_offset.virtual_pointer {
                        tabix.first_feature_offset = chunk.begin_vp;
                    }
                }

                tabix_bin.chunks = chunks;
                chr_index.bins.insert(tabix_bin.bin, tabix_bin);
            }

            let n_intv = read_i32(&bytes, i)?;
            i += 4;

            let mut intervals = Vec::with_capacity(n_intv as usize);

            for _ in 0..n_intv {
                let interval = read_u64(&bytes, i)?;
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

impl SamIndex for TabixIndex {}

fn read_slice(bytes: &[u8], begin: usize, length: usize) -> Result<&[u8], ParsingError> {
    let end = begin.checked_add(length).ok_or(ParsingError::InsufficientBytes)?;
    bytes.get(begin..end).ok_or(ParsingError::InsufficientBytes)
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, ParsingError> {
    let slice = read_slice(bytes, offset, 4)?;
    Ok(i32::from_le_bytes(slice.try_into()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ParsingError> {
    let slice = read_slice(bytes, offset, 4)?;
    Ok(u32::from_le_bytes(slice.try_into()?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ParsingError> {
    let slice = read_slice(bytes, offset, 8)?;
    Ok(u64::from_le_bytes(slice.try_into()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le_i32(value: i32) -> Vec<u8> {
        value.to_le_bytes().to_vec()
    }

    fn header_bytes() -> Vec<u8> {
        let names = b"chr1\0".to_vec();
        let name_length = names.len() as i32;
        [
            b"TBI\x01".to_vec(),
            le_i32(1),
            le_i32(2),
            le_i32(1),
            le_i32(2),
            le_i32(0),
            le_i32(35),
            le_i32(0),
            le_i32(name_length),
            names,
        ]
        .concat()
    }

    fn reference_bytes() -> Vec<u8> {
        [
            le_i32(1),
            4681u32.to_le_bytes().to_vec(),
            le_i32(1),
            0u64.to_le_bytes().to_vec(),
            65536u64.to_le_bytes().to_vec(),
            le_i32(1),
            0u64.to_le_bytes().to_vec(),
        ]
        .concat()
    }

    fn index_bytes() -> Vec<u8> {
        [header_bytes(), reference_bytes()].concat()
    }

    fn truncated(bytes: Vec<u8>, length: usize) -> Vec<u8> {
        bytes.into_iter().take(length).collect()
    }

    #[test]
    fn complete_index_parses() {
        let tabix = TabixIndex::from_bytes(index_bytes()).unwrap();
        assert_eq!(tabix.names, ["chr1"]);
        assert_eq!(tabix.references.len(), 1);
    }

    #[test]
    fn empty_bytes_return_an_error() {
        let result = TabixIndex::from_bytes(Vec::new());
        assert!(matches!(result, Err(ParsingError::InsufficientBytes)));
    }

    #[test]
    fn truncated_header_returns_an_error() {
        let bytes = truncated(index_bytes(), 10);
        let result = TabixIndex::from_bytes(bytes);
        assert!(matches!(result, Err(ParsingError::InsufficientBytes)));
    }

    #[test]
    fn truncated_names_return_an_error() {
        let header_length = header_bytes().len();
        let bytes = truncated(index_bytes(), header_length - 2);
        let result = TabixIndex::from_bytes(bytes);
        assert!(matches!(result, Err(ParsingError::InsufficientBytes)));
    }

    #[test]
    fn truncated_body_returns_an_error() {
        let full_length = index_bytes().len();
        let bytes = truncated(index_bytes(), full_length - 4);
        let result = TabixIndex::from_bytes(bytes);
        assert!(matches!(result, Err(ParsingError::InsufficientBytes)));
    }

    #[test]
    fn truncated_chunk_returns_an_error() {
        let header_length = header_bytes().len();
        let bytes = truncated(index_bytes(), header_length + 16);
        let result = TabixIndex::from_bytes(bytes);
        assert!(matches!(result, Err(ParsingError::InsufficientBytes)));
    }
}

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

use crate::indexes::bigwig::r_tree::RTreeError;

#[derive(Debug)]
pub struct RTreeHeader {
    pub magic: u32,
    pub block_size: u32,
    pub item_count: u64,
    pub start_chrom_idx: u32,
    pub start_base: u32,
    pub end_chrom_idx: u32,
    pub end_base: u32,
    pub end_file_offset: u64,
    pub items_per_slot: u32,
    pub reserved: u32
}

impl RTreeHeader {

    pub const SIZE: usize = 48;

    pub fn new() -> Self {
        RTreeHeader {
            magic: 0x52545245, // "RTree" in ASCII
            block_size: 0,
            item_count: 0,
            start_chrom_idx: 0,
            start_base: 0,
            end_chrom_idx: 0,
            end_base: 0,
            end_file_offset: 0,
            items_per_slot: 0,
            reserved: 0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RTreeError> {
        if bytes.len() < RTreeHeader::SIZE {
            return Err(RTreeError::RTreeReadError("Not enough bytes for a complete RTree header".into()));
        }

        let magic = u32::from_le_bytes(bytes[0..4].try_into()?);
        let block_size = u32::from_le_bytes(bytes[4..8].try_into()?);
        let item_count = u64::from_le_bytes(bytes[8..16].try_into()?);
        let start_chrom_idx = u32::from_le_bytes(bytes[16..20].try_into()?);
        let start_base = u32::from_le_bytes(bytes[20..24].try_into()?);
        let end_chrom_idx = u32::from_le_bytes(bytes[24..28].try_into()?);
        let end_base = u32::from_le_bytes(bytes[28..32].try_into()?);
        let end_file_offset = u64::from_le_bytes(bytes[32..40].try_into()?);
        let items_per_slot = u32::from_le_bytes(bytes[40..44].try_into()?);
        let reserved = u32::from_le_bytes(bytes[44..48].try_into()?);

        assert!(reserved == 0, "RTree Header Reserved byte should be zero, found: {}", reserved);
        
        Ok(RTreeHeader {
            magic,
            block_size,
            item_count,
            start_chrom_idx,
            start_base,
            end_chrom_idx,
            end_base,
            end_file_offset,
            items_per_slot,
            reserved,
        })
    }
}
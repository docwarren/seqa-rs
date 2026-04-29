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

use std::u32;
use thiserror::Error;
use core::array::TryFromSliceError;
use super::overlaps::Overlaps;
use crate::indexes::bigwig::r_tree::r_tree_node::RTreeNode;

#[derive(Debug, Clone, Error)]
pub enum RTreeNonLeafError {
    #[error("Failed to read RTree non leaf: {0}")]
    ReadError(String),

    #[error("Parsing error: {0}")]
    ParseError(#[from] TryFromSliceError),
}


#[derive(Debug, Clone)]
pub struct RTreeNonLeaf {
    pub start_chrom_idx: u32,
    pub start_base: u32,
    pub end_chrom_idx: u32,
    pub end_base: u32,
    pub child_node_offset: u64,
    pub child: RTreeNode, // The child node that this non-leaf node points to
}

impl RTreeNonLeaf {

    pub const SIZE: usize = 24; // Size of the RTreeNonLeaf in bytes

    pub fn new() -> Self {
        RTreeNonLeaf {
            start_chrom_idx: u32::MAX,
            start_base: u32::MAX,
            end_chrom_idx: u32::MAX,
            end_base: u32::MAX,
            child_node_offset: u32::MAX as u64,
            child: RTreeNode::new(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RTreeNonLeafError> {
        if bytes.len() < RTreeNonLeaf::SIZE {
            return Err(RTreeNonLeafError::ReadError("Not enough bytes for a complete RTree non-leaf".into()));
        }

        let start_chrom_idx = u32::from_le_bytes(bytes[0..4].try_into()?);
        let start_base = u32::from_le_bytes(bytes[4..8].try_into()?);
        let end_chrom_idx = u32::from_le_bytes(bytes[8..12].try_into()?);
        let end_base = u32::from_le_bytes(bytes[12..16].try_into()?);
        let child_node_offset = u64::from_le_bytes(bytes[16..24].try_into()?);

        Ok(RTreeNonLeaf {
            start_chrom_idx,
            start_base,
            end_chrom_idx,
            end_base,
            child_node_offset,
            child: RTreeNode::new(),
        })
    }
}

impl Overlaps for RTreeNonLeaf {
    fn overlaps(&self, chr_id1: u32, chr_id2: u32, start: u32, end: u32) -> bool {
        ((chr_id2 > self.start_chrom_idx) || (chr_id2 == self.start_chrom_idx && end >= self.start_base)) &&
        ((chr_id1 < self.end_chrom_idx) || (chr_id1 == self.end_chrom_idx && start <= self.end_base))
    }
}
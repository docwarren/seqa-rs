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

pub mod r_tree_header;
pub mod r_tree_leaf;
pub mod r_tree_node;
pub mod r_tree_non_leaf;
pub mod overlaps;

use crate::{
    codecs::bgzip,
    indexes::bigwig::r_tree::{
        r_tree_leaf::RTreeLeaf,
        r_tree_node::{RTreeNode, RTreeNodeError, RTreeNodeType},
    },
    stores::{StoreService, error::StoreError},
};
use core::array::TryFromSliceError;
use overlaps::Overlaps;
use r_tree_header::RTreeHeader;
use std::ops::Range;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RTreeError {
    #[error("Failed to read RTree file: {0}")]
    RTreeReadError(String),

    #[error("StoreError: {0}")]
    StoreError(#[from] StoreError),

    #[error("BgZip Error: {0}")]
    BgZipError(#[from] bgzip::BgZipError),

    #[error("Parsing Error: {0}")]
    ParsingError(#[from] TryFromSliceError),

    #[error("RTree Node Error: {0}")]
    RTreeNodeError(#[from] RTreeNodeError),
}

pub struct RTree {
    pub header: RTreeHeader,
    pub offset: u64,
    pub root_offset: usize,
    pub root: RTreeNode,
}

impl RTree {
    pub fn new() -> Self {
        RTree {
            header: RTreeHeader::new(),
            offset: 0,
            root_offset: 0,
            root: RTreeNode::new(),
        }
    }

    pub async fn from_file(
        store: &StoreService,
        file_path: &str,
        range: Range<u64>,
    ) -> Result<RTree, RTreeError> {
        let tree_range = range.start..range.end;
        let tree_bytes = store.get_range(&file_path, tree_range).await?;
        let index_header = RTreeHeader::from_bytes(&tree_bytes)?;
        let root_offset = range.start as usize + RTreeHeader::SIZE;
        let root = RTreeNode::from_bytes(&tree_bytes[RTreeHeader::SIZE..], root_offset)?;

        Ok(RTree {
            header: index_header,
            offset: range.start,
            root_offset,
            root,
        })
    }

    pub fn get_overlapping_leaves(&self, chr_id: u32, begin: u32, end: u32) -> Vec<&RTreeLeaf> {
        let leaves = get_overlapping_leaves(&self.root, chr_id, begin, end);
        leaves
    }
}

pub fn get_overlapping_leaves(
    node: &RTreeNode,
    chr_id: u32,
    begin: u32,
    end: u32,
) -> Vec<&RTreeLeaf> {
    let mut leaves = Vec::new();
    for child in &node.children {
        match child {
            RTreeNodeType::Leaf(leaf) => {
                if leaf.overlaps(chr_id, chr_id, begin, end) {
                    leaves.push(leaf);
                }
            }
            RTreeNodeType::NonLeaf(non_leaf) => {
                if non_leaf.overlaps(chr_id, chr_id, begin, end) {
                    leaves.extend(get_overlapping_leaves(&non_leaf.child, chr_id, begin, end))
                }
            }
        }
    }
    leaves
}

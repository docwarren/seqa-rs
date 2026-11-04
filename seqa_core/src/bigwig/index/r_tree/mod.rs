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
use crate::api::search_error::SearchError;
use crate::bigwig::index::r_tree::{
    r_tree_leaf::RTreeLeaf,
    r_tree_node::{RTreeNode, RTreeNodeType},
};
use crate::coordinate::Coordinate;
use crate::stores::StoreService;
use overlaps::Overlaps;
use r_tree_header::RTreeHeader;
use std::ops::Range;

pub mod overlaps;
pub mod r_tree_header;
pub mod r_tree_leaf;
pub mod r_tree_node;
pub mod r_tree_non_leaf;

#[derive(Default)]
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
    ) -> Result<RTree, SearchError> {
        let tree_range = range.start..range.end;
        let tree_bytes =
            store
                .get_range(file_path, tree_range)
                .await
                .map_err(|e| SearchError::ReadError {
                    path: file_path.to_string(),
                    source: e,
                })?;
        let index_header =
            RTreeHeader::from_bytes(&tree_bytes).map_err(|e| SearchError::ParseError {
                source: e,
                path: file_path.to_string(),
            })?;
        let root_offset = range.start as usize + RTreeHeader::SIZE;
        let root =
            RTreeNode::from_bytes(&tree_bytes[RTreeHeader::SIZE..], root_offset).map_err(|e| {
                SearchError::ParseError {
                    path: file_path.to_string(),
                    source: e,
                }
            })?;

        Ok(RTree {
            header: index_header,
            offset: range.start,
            root_offset,
            root,
        })
    }

    pub fn get_overlapping_leaves(&self, chr_id: u32, coordinate: &Coordinate) -> Vec<RTreeLeaf> {
        get_overlapping_leaves(&self.root, chr_id, coordinate)
    }
}

pub fn get_overlapping_leaves(
    node: &RTreeNode,
    chr_id: u32,
    coordinate: &Coordinate,
) -> Vec<RTreeLeaf> {
    let mut leaves = Vec::new();
    for child in &node.children {
        match child {
            RTreeNodeType::Leaf(leaf) => {
                if leaf.overlaps(chr_id, chr_id, coordinate.get_begin(), coordinate.get_end()) {
                    leaves.push(leaf.clone());
                }
            }
            RTreeNodeType::NonLeaf(non_leaf) => {
                if non_leaf.overlaps(chr_id, chr_id, coordinate.get_begin(), coordinate.get_end()) {
                    leaves.extend(get_overlapping_leaves(&non_leaf.child, chr_id, coordinate))
                }
            }
        }
    }
    leaves
}

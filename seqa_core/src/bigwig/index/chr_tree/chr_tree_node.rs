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

use crate::api::parsing_error::ParsingError;
use crate::bigwig::index::chr_tree::chr_tree_leaf::ChrTreeLeaf;
use crate::bigwig::index::chr_tree::chr_tree_non_leaf::ChrTreeNonLeaf;

#[derive(Debug)]
pub enum ChrTreeChild {
    NonLeaf(ChrTreeNonLeaf),
    Leaf(ChrTreeLeaf),
}

fn read_leaf_nodes(
    bytes: &[u8],
    start_offset: usize,
    count: usize,
    key_size: u32,

) -> Result<(Vec<ChrTreeChild>, usize), ParsingError> {
    if bytes.len() < start_offset + count * 8 {
        return Err(ParsingError::InsufficientBytes);
    }

    let mut nodes: Vec<ChrTreeChild> = Vec::new();
    let mut bytes_read = 0;
    let mut offset = start_offset;
    
    for _ in 0..count {

        let (item, next_bytes_read) = ChrTreeLeaf::from_bytes(bytes, offset, key_size)?;
        nodes.push(ChrTreeChild::Leaf(item));
        offset += next_bytes_read;
        bytes_read += next_bytes_read;
    }
    Ok((nodes, bytes_read))
}

fn read_non_leaf_nodes(
    bytes: &[u8],
    start_offset: usize,
    count: usize,
    key_size: u32,

) -> Result<(Vec<ChrTreeChild>, usize), ParsingError> {

    if bytes.len() < start_offset + count * 12 {
        return Err(ParsingError::InsufficientBytes);
    }

    let mut nodes: Vec<ChrTreeChild> = Vec::with_capacity(count);
    let mut offset = start_offset;
    let mut bytes_read = 0;

    for _ in 0..count {

        let (node, next_bytes_read) = ChrTreeNonLeaf::from_bytes(bytes, offset, key_size)?;
        nodes.push(ChrTreeChild::NonLeaf(node));
        offset += next_bytes_read;
        bytes_read += next_bytes_read;
    }
    Ok((nodes, bytes_read))
}

#[derive(Debug)]
pub struct ChrTreeNode {
    pub is_leaf: bool,
    pub reserved: u8,
    pub count: u16,
    pub children: Vec<ChrTreeChild>,
}

impl Default for ChrTreeNode {
    fn default() -> Self {
        Self::new()
    }
}

impl ChrTreeNode {
    
    pub fn new() -> Self {
        ChrTreeNode {
            is_leaf: false,
            reserved: 0,
            count: 0,
            children: Vec::new(),
        }
    }

    pub fn from_bytes(bytes: &[u8], offset: usize, key_size: u32) -> Result<(Self, usize), ParsingError> {

        if bytes.len() < 4 {
            return Err(ParsingError::InsufficientBytes);
        }

        let is_leaf = bytes[offset] != 0;
        let reserved = bytes[offset + 1];
        let count = u16::from_le_bytes(bytes[offset + 2..offset + 4].try_into()?);

        assert_eq!(reserved, 0, "Node Reserved byte should be zero, found: {}", reserved);

        let (nodes, bytes_read) = if is_leaf {
            read_leaf_nodes(bytes, offset + 4, count as usize, key_size)?
        } else {
            read_non_leaf_nodes(bytes, offset + 4, count as usize, key_size)?
        };

        Ok((ChrTreeNode {
            is_leaf,
            reserved,
            count,
            children: nodes,
        }, bytes_read + 4))
    }
}

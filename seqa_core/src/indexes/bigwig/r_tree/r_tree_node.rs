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

use thiserror::Error;
use crate::indexes::bigwig::r_tree::r_tree_leaf::{RTreeLeaf, RTreeLeafError};
use crate::indexes::bigwig::r_tree::r_tree_non_leaf::{RTreeNonLeaf, RTreeNonLeafError};

#[derive(Debug, Error)]
pub enum RTreeNodeError {
    #[error("Failed to read RTree node: {0}")]
    RTreeNodeReadError(String),

    #[error("Error reading RTree Leaf: {0}")]
    RTreeLeafReadError(#[from] RTreeLeafError),

    #[error("Error reading RTree Non Leaf: {0}")]
    RTreeNonLeafReadError(#[from] RTreeNonLeafError),
}

#[derive(Debug, Clone)]
pub enum RTreeNodeType {
    Leaf(RTreeLeaf),
    NonLeaf(RTreeNonLeaf),
}

#[derive(Debug, Clone)]
pub struct RTreeNode {
    pub is_leaf: bool,
    pub reserved: u8,
    pub count: u16,
    pub children: Vec<RTreeNodeType>,
}

impl RTreeNode {
    pub const SIZE: usize = 4; // is_leaf + reserved + count

    pub fn new() -> Self {
        RTreeNode {
            is_leaf: false,
            reserved: 0,
            count: 0,
            children: Vec::new(),
        }
    }

    pub fn from_bytes(bytes: &[u8], root_offset: usize) -> Result<Self, RTreeNodeError> {
        let is_leaf = bytes[0] != 0;
        let reserved = bytes[1];
        let count = u16::from_le_bytes(bytes[2..4].try_into().map_err(|_| RTreeNodeError::RTreeNodeReadError("Invalid R Tree Node".into()))?);

        assert!(reserved == 0, "Reserved byte should be zero, found: {}", reserved);

        let mut children = Vec::new();
        let mut offset = RTreeNode::SIZE;

        if is_leaf {
            for _ in 0..count {
                let leaf_range = offset..offset + RTreeLeaf::SIZE;
                let leaf = RTreeLeaf::from_bytes(&bytes[leaf_range])?;
                children.push(RTreeNodeType::Leaf(leaf));
                offset += RTreeLeaf::SIZE;
            }
        } else {
            for _ in 0..count {
                let non_leaf_range = offset..offset + RTreeNonLeaf::SIZE;
                let mut non_leaf = RTreeNonLeaf::from_bytes(&bytes[non_leaf_range])?;

                let child_start = non_leaf.child_node_offset as usize;
                let child_range = (child_start - root_offset)..;
                let child_bytes = &bytes[child_range];
                non_leaf.child = RTreeNode::from_bytes(child_bytes, child_start)?;
                
                children.push(RTreeNodeType::NonLeaf(non_leaf));
                offset += RTreeNonLeaf::SIZE;
            }
        }

        Ok(RTreeNode {
            is_leaf,
            reserved,
            count,
            children,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: write a non-leaf item (24 bytes) into buf at the given offset.
    fn write_non_leaf(buf: &mut Vec<u8>, offset: usize, start_chrom: u32, start_base: u32, end_chrom: u32, end_base: u32, child_offset: u64) {
        buf[offset..offset+4].copy_from_slice(&start_chrom.to_le_bytes());
        buf[offset+4..offset+8].copy_from_slice(&start_base.to_le_bytes());
        buf[offset+8..offset+12].copy_from_slice(&end_chrom.to_le_bytes());
        buf[offset+12..offset+16].copy_from_slice(&end_base.to_le_bytes());
        buf[offset+16..offset+24].copy_from_slice(&child_offset.to_le_bytes());
    }

    /// Helper: write a node header (4 bytes) into buf at the given offset.
    fn write_node_header(buf: &mut Vec<u8>, offset: usize, is_leaf: bool, count: u16) {
        buf[offset] = if is_leaf { 1 } else { 0 };
        buf[offset+1] = 0; // reserved
        buf[offset+2..offset+4].copy_from_slice(&count.to_le_bytes());
    }

    /// Helper: write a leaf item (32 bytes) into buf at the given offset.
    fn write_leaf(buf: &mut Vec<u8>, offset: usize, start_chrom: u32, start_base: u32, end_chrom: u32, end_base: u32, data_offset: u64, data_size: u64) {
        buf[offset..offset+4].copy_from_slice(&start_chrom.to_le_bytes());
        buf[offset+4..offset+8].copy_from_slice(&start_base.to_le_bytes());
        buf[offset+8..offset+12].copy_from_slice(&end_chrom.to_le_bytes());
        buf[offset+12..offset+16].copy_from_slice(&end_base.to_le_bytes());
        buf[offset+16..offset+24].copy_from_slice(&data_offset.to_le_bytes());
        buf[offset+24..offset+32].copy_from_slice(&data_size.to_le_bytes());
    }

    #[test]
    fn parse_single_leaf_node() {
        // A leaf node with 2 leaf items
        let root_offset: usize = 500;
        let mut buf = vec![0u8; 4 + 2 * RTreeLeaf::SIZE];

        write_node_header(&mut buf, 0, true, 2);
        write_leaf(&mut buf, 4, 0, 100, 0, 200, 9000, 50);
        write_leaf(&mut buf, 4 + RTreeLeaf::SIZE, 0, 300, 0, 400, 9050, 60);

        let node = RTreeNode::from_bytes(&buf, root_offset).unwrap();
        assert!(node.is_leaf);
        assert_eq!(node.count, 2);
        assert_eq!(node.children.len(), 2);

        if let RTreeNodeType::Leaf(ref leaf) = node.children[0] {
            assert_eq!(leaf.start_base, 100);
            assert_eq!(leaf.end_base, 200);
            assert_eq!(leaf.data_offset, 9000);
        } else {
            panic!("Expected leaf");
        }
    }

    #[test]
    fn parse_two_level_tree() {
        // Root (non-leaf, 1 child) → Leaf node (1 leaf item)
        // Layout in bytes:
        //   [0..4]    root header: non-leaf, count=1
        //   [4..28]   non-leaf item pointing to child at absolute offset root_offset+28
        //   [28..32]  leaf node header: leaf, count=1
        //   [32..64]  leaf item
        let root_offset: usize = 1000;
        let buf_size = 4 + RTreeNonLeaf::SIZE + 4 + RTreeLeaf::SIZE;
        let mut buf = vec![0u8; buf_size];

        let child_abs_offset = root_offset + 28;

        write_node_header(&mut buf, 0, false, 1);
        write_non_leaf(&mut buf, 4, 0, 0, 0, 1000, child_abs_offset as u64);
        write_node_header(&mut buf, 28, true, 1);
        write_leaf(&mut buf, 32, 0, 50, 0, 150, 8000, 100);

        let node = RTreeNode::from_bytes(&buf, root_offset).unwrap();
        assert!(!node.is_leaf);
        assert_eq!(node.count, 1);

        if let RTreeNodeType::NonLeaf(ref non_leaf) = node.children[0] {
            assert_eq!(non_leaf.start_base, 0);
            assert_eq!(non_leaf.end_base, 1000);
            assert!(non_leaf.child.is_leaf);
            assert_eq!(non_leaf.child.count, 1);
            if let RTreeNodeType::Leaf(ref leaf) = non_leaf.child.children[0] {
                assert_eq!(leaf.start_base, 50);
                assert_eq!(leaf.end_base, 150);
                assert_eq!(leaf.data_offset, 8000);
            } else {
                panic!("Expected leaf in child");
            }
        } else {
            panic!("Expected non-leaf");
        }
    }

    #[test]
    fn parse_three_level_tree() {
        // This is the case that triggered the bug: root → internal → leaf.
        // With the old code, the recursive call at the third level would use
        // the root's offset instead of the internal node's offset, reading
        // from the wrong position in the byte buffer.
        //
        // Layout:
        //   [0..4]    root: non-leaf, count=1
        //   [4..28]   non-leaf item → child at offset 28
        //   [28..32]  internal: non-leaf, count=1
        //   [32..56]  non-leaf item → child at offset 56
        //   [56..60]  leaf: leaf, count=1
        //   [60..92]  leaf item
        let root_offset: usize = 2000;
        let buf_size = 4 + RTreeNonLeaf::SIZE + 4 + RTreeNonLeaf::SIZE + 4 + RTreeLeaf::SIZE;
        let mut buf = vec![0u8; buf_size];

        let internal_abs = root_offset + 28;
        let leaf_abs = root_offset + 56;

        // Root node
        write_node_header(&mut buf, 0, false, 1);
        write_non_leaf(&mut buf, 4, 0, 0, 0, 5000, internal_abs as u64);

        // Internal node
        write_node_header(&mut buf, 28, false, 1);
        write_non_leaf(&mut buf, 32, 0, 100, 0, 2000, leaf_abs as u64);

        // Leaf node
        write_node_header(&mut buf, 56, true, 1);
        write_leaf(&mut buf, 60, 0, 100, 0, 200, 7000, 256);

        let node = RTreeNode::from_bytes(&buf, root_offset).unwrap();
        assert!(!node.is_leaf);
        assert_eq!(node.count, 1);

        // Traverse root → internal → leaf
        let internal = match &node.children[0] {
            RTreeNodeType::NonLeaf(n) => n,
            _ => panic!("Expected non-leaf at level 1"),
        };
        assert!(!internal.child.is_leaf);
        assert_eq!(internal.child.count, 1);

        let leaf_parent = match &internal.child.children[0] {
            RTreeNodeType::NonLeaf(n) => n,
            _ => panic!("Expected non-leaf at level 2"),
        };
        assert!(leaf_parent.child.is_leaf);
        assert_eq!(leaf_parent.child.count, 1);

        let leaf = match &leaf_parent.child.children[0] {
            RTreeNodeType::Leaf(l) => l,
            _ => panic!("Expected leaf at level 3"),
        };
        assert_eq!(leaf.start_base, 100);
        assert_eq!(leaf.end_base, 200);
        assert_eq!(leaf.data_offset, 7000);
        assert_eq!(leaf.data_size, 256);
    }

    #[test]
    fn parse_three_level_tree_with_large_root_offset() {
        // Use a large root_offset to simulate a tree deep into a file.
        // The bug was that child_node_offset - root_offset was used as an
        // index into a sub-sliced buffer, but root_offset wasn't updated
        // for the sub-slice. With a large root_offset the mis-indexing
        // would read well past the buffer end or into garbage bytes.
        let root_offset: usize = 1_000_000;
        let buf_size = 4 + RTreeNonLeaf::SIZE + 4 + RTreeNonLeaf::SIZE + 4 + RTreeLeaf::SIZE;
        let mut buf = vec![0u8; buf_size];

        let internal_abs = root_offset + 28;
        let leaf_abs = root_offset + 56;

        write_node_header(&mut buf, 0, false, 1);
        write_non_leaf(&mut buf, 4, 0, 0, 0, 10000, internal_abs as u64);

        write_node_header(&mut buf, 28, false, 1);
        write_non_leaf(&mut buf, 32, 0, 0, 0, 5000, leaf_abs as u64);

        write_node_header(&mut buf, 56, true, 1);
        write_leaf(&mut buf, 60, 0, 500, 0, 600, 99000, 512);

        let node = RTreeNode::from_bytes(&buf, root_offset).unwrap();

        // Walk to the deepest leaf
        let internal = match &node.children[0] {
            RTreeNodeType::NonLeaf(n) => n,
            _ => panic!("Expected non-leaf"),
        };
        let leaf_parent = match &internal.child.children[0] {
            RTreeNodeType::NonLeaf(n) => n,
            _ => panic!("Expected non-leaf"),
        };
        let leaf = match &leaf_parent.child.children[0] {
            RTreeNodeType::Leaf(l) => l,
            _ => panic!("Expected leaf"),
        };
        assert_eq!(leaf.start_base, 500);
        assert_eq!(leaf.end_base, 600);
        assert_eq!(leaf.data_size, 512);
    }

    #[test]
    fn parse_branching_non_leaf_node() {
        // Root with 2 non-leaf children, each pointing to a leaf node.
        // Tests that sibling child offsets are resolved correctly.
        let root_offset: usize = 3000;
        // root header(4) + 2 non-leaf items(48) + leaf1 header(4) + leaf1 item(32) + leaf2 header(4) + leaf2 item(32) = 124
        let buf_size = 4 + 2 * RTreeNonLeaf::SIZE + 2 * (4 + RTreeLeaf::SIZE);
        let mut buf = vec![0u8; buf_size];

        let leaf1_abs = root_offset + 4 + 2 * RTreeNonLeaf::SIZE; // 3052
        let leaf2_abs = leaf1_abs + 4 + RTreeLeaf::SIZE;           // 3088

        write_node_header(&mut buf, 0, false, 2);
        write_non_leaf(&mut buf, 4, 0, 0, 0, 500, leaf1_abs as u64);
        write_non_leaf(&mut buf, 28, 0, 500, 0, 1000, leaf2_abs as u64);

        let leaf1_buf_offset = leaf1_abs - root_offset;
        write_node_header(&mut buf, leaf1_buf_offset, true, 1);
        write_leaf(&mut buf, leaf1_buf_offset + 4, 0, 10, 0, 20, 4000, 64);

        let leaf2_buf_offset = leaf2_abs - root_offset;
        write_node_header(&mut buf, leaf2_buf_offset, true, 1);
        write_leaf(&mut buf, leaf2_buf_offset + 4, 0, 600, 0, 700, 5000, 128);

        let node = RTreeNode::from_bytes(&buf, root_offset).unwrap();
        assert_eq!(node.count, 2);

        let child1 = match &node.children[0] {
            RTreeNodeType::NonLeaf(n) => n,
            _ => panic!("Expected non-leaf"),
        };
        let leaf1 = match &child1.child.children[0] {
            RTreeNodeType::Leaf(l) => l,
            _ => panic!("Expected leaf"),
        };
        assert_eq!(leaf1.start_base, 10);
        assert_eq!(leaf1.data_offset, 4000);

        let child2 = match &node.children[1] {
            RTreeNodeType::NonLeaf(n) => n,
            _ => panic!("Expected non-leaf"),
        };
        let leaf2 = match &child2.child.children[0] {
            RTreeNodeType::Leaf(l) => l,
            _ => panic!("Expected leaf"),
        };
        assert_eq!(leaf2.start_base, 600);
        assert_eq!(leaf2.data_offset, 5000);
    }
}
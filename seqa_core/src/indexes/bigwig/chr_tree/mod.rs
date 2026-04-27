use std::collections::HashMap;
use core::array::TryFromSliceError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use chr_tree_header::ChrTreeHeader;
use crate::indexes::bigwig::{chr_tree::{chr_tree_header::ChrTreeHeaderError, chr_tree_node::{ChrTreeChild, ChrTreeNode, ChrTreeNodeError}}, header::{BigwigHeader}};

pub mod chr_tree_header;
pub mod chr_tree_node;
pub mod chr_tree_leaf;
pub mod chr_tree_non_leaf;

#[derive(Debug, Error, Clone)]
pub enum ChrTreeError {
    #[error("Parsing Error: {0}")]
    InvalidData(#[from] TryFromSliceError),

    #[error("ChromTreeNode Error: {0}")]
    NodeError(#[from] ChrTreeNodeError),

    #[error("Chromosome Tree Header Error: {0}")]
    HeaderError(#[from] ChrTreeHeaderError),
}

pub fn read_tree(root: &ChrTreeNode) -> (HashMap<String, u32>, HashMap<u32, String>) {
    let mut key_map = HashMap::new();
    let mut name_map = HashMap::new();

    for child in &root.children {
        match child {
            ChrTreeChild::Leaf(leaf) => {
                key_map.insert(leaf.key.clone(), leaf.chr_id);
                name_map.insert(leaf.chr_id, leaf.key.clone());
            }
            ChrTreeChild::NonLeaf(non_leaf) => {
                let (sub_key_map, sub_name_map) = read_tree(&non_leaf.child);
                key_map.extend(sub_key_map);
                name_map.extend(sub_name_map);
            }
        }
    }
    (key_map, name_map)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BigwigChrTree {
    pub header: ChrTreeHeader,
    pub key_map: HashMap<String, u32>, // Maps chromosome names to their indices
    pub name_map: HashMap<u32, String>, // Maps chromosome indices to their names
}

impl BigwigChrTree {

    pub fn new() -> Self {
        BigwigChrTree {
            header: ChrTreeHeader::new(),
            key_map: HashMap::new(),
            name_map: HashMap::new(),
        }
    }

    pub fn from_bytes(bytes: &[u8], header: &BigwigHeader) -> Result<Self, ChrTreeError> {
        let chr_tree_range = header.chromosome_tree_offset as usize..header.chromosome_tree_offset as usize + ChrTreeHeader::SIZE as usize;
        let chr_tree_header = ChrTreeHeader::from_bytes(&bytes[chr_tree_range.clone()])?;
        let (root, _) = ChrTreeNode::from_bytes(bytes, chr_tree_range.end, chr_tree_header.key_size)?;
        let (key_map, name_map) = read_tree(&root);

        Ok(BigwigChrTree { header: chr_tree_header, key_map, name_map })
    }

    pub fn get_chromosome_id(&self, key: &str) -> Option<u32> {
        crate::genome::chromosome_aliases(key)
            .iter()
            .find_map(|alias| self.key_map.get(alias).cloned())
    }

    pub fn get_chromosome_name(&self, chr_id: u32) -> Option<String> {
        self.name_map.get(&chr_id).cloned()
    }
}


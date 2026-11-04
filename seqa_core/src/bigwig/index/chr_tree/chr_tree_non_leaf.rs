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
use crate::bigwig::index::chr_tree::chr_tree_node::ChrTreeNode;

#[derive(Debug, Default)]
pub struct ChrTreeNonLeaf {
    pub key: String,
    pub child_offset: u64,
    pub child: ChrTreeNode,
}

impl ChrTreeNonLeaf {
    pub fn new() -> Self {
        ChrTreeNonLeaf {
            key: String::new(),
            child_offset: 0,
            child: ChrTreeNode::new(),
        }
    }

    pub fn from_bytes(
        bytes: &[u8],
        start_offset: usize,
        key_size: u32,

    ) -> Result<(Self, usize), ParsingError> {

        if bytes.len() < 8 + key_size as usize {
            return Err(ParsingError::InsufficientBytes);
        }
        let offset = start_offset;

        let key_bytes= &bytes[offset..offset + key_size as usize]
            .iter()
            .filter(|&&b| b != 0)
            .cloned()
            .collect::<Vec<u8>>();
        
        let key = String::from_utf8_lossy(key_bytes).to_string();
        let range = offset + key_size as usize..offset + key_size as usize + 8;
        let child_offset = u64::from_le_bytes(bytes[range].try_into()?);
        let (child, _) = ChrTreeNode::from_bytes(bytes, child_offset as usize, key_size)?;

        Ok((
            ChrTreeNonLeaf {
                key,
                child_offset,
                child,
            },
            key_size as usize + 8,
        ))
    }
}

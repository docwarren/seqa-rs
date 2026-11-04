use crate::api::parsing_error::ParsingError;

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
#[derive(Debug, Default)]
pub struct ChrTreeLeaf {
    pub key: String,
    pub chr_id: u32,
    pub chr_size: u32
}

impl ChrTreeLeaf {
    
    pub fn new() -> Self {
        ChrTreeLeaf {
            key: String::new(),
            chr_id: 0,
            chr_size: 0
        }
    }

    pub fn from_bytes(bytes: &[u8], offset: usize, key_size: u32) -> Result<(Self, usize), ParsingError> {
        if bytes.len() < 8 + key_size as usize {
            return Err(ParsingError::InsufficientBytes);
        }
        let key = bytes[offset..offset + key_size as usize].iter().filter(|&&b| b != 0).cloned().collect::<Vec<u8>>();
        let key = String::from_utf8_lossy(&key).to_string();

        let range = offset + key_size as usize..offset + key_size as usize + 4;
        let chr_id = u32::from_le_bytes(bytes[range].try_into()?);
        
        let range = offset + key_size as usize + 4..offset + key_size as usize + 8;
        let chr_size = u32::from_le_bytes(bytes[range].try_into()?);

        let bytes_read = key_size as usize + 4 + 4;

        Ok((ChrTreeLeaf {
            key,
            chr_id,
            chr_size
        }, bytes_read))
    }
}
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
use thiserror::Error;
use core::array::TryFromSliceError;

#[derive(Debug, Error, Clone)]
pub enum ChrTreeHeaderError {
    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Parsing error: {0}")]
    ParsingError(#[from] TryFromSliceError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChrTreeHeader {
    _magic: String, // 4 bytes
    _block_size: u32,
    pub key_size: u32,
    _val_size: u32,
    _item_count: u64,
    _reserved: u64,
}

impl ChrTreeHeader {
    pub const SIZE: usize = 32;

    pub fn new() -> Self {
        ChrTreeHeader {
            _magic: String::from("CHRT"),
            _block_size: 0,
            key_size: 0,
            _val_size: 0,
            _item_count: 0,
            _reserved: 0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ChrTreeHeaderError> {

        if bytes.len() < ChrTreeHeader::SIZE {
            return Err(ChrTreeHeaderError::InvalidData("Not enough bytes for a complete chromosome tree header".into()));
        }

        let magic = String::from_utf8_lossy(&bytes[0..4]).to_string();
        let block_size = u32::from_le_bytes(bytes[4..8].try_into()?);
        let key_size = u32::from_le_bytes(bytes[8..12].try_into()?);
        let val_size = u32::from_le_bytes(bytes[12..16].try_into()?);
        let item_count = u64::from_le_bytes(bytes[16..24].try_into()?);
        let reserved = u64::from_le_bytes(bytes[24..32].try_into()?);

        assert!(reserved == 0, "Chromosome Tree Header Reserved byte should be zero, found: {}", reserved);

        Ok(ChrTreeHeader {
            _magic: magic,
            _block_size: block_size,
            key_size,
            _val_size: val_size,
            _item_count: item_count,
            _reserved: reserved,
        })
    }
}
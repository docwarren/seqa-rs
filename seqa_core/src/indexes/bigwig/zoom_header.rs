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

#[derive(Debug, Error)]
pub enum ZoomHeaderError {
    #[error("Failed to parse ZoomHeader: {0}")]
    HeaderError(String),

    #[error("Parsing error: {0}")]
    ParseError(#[from] core::array::TryFromSliceError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoomHeader {
    pub reduction_level: u32,
    pub reserved: u32,
    pub data_offset: u64,
    pub index_offset: u64
}

impl ZoomHeader {
    pub fn new() -> Self {
        ZoomHeader {
            reduction_level: 0,
            reserved: 0,
            data_offset: 0,
            index_offset: 0
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ZoomHeaderError> {
        if bytes.len() < 24 {
            return Err(ZoomHeaderError::HeaderError("Not enough bytes for a complete zoom header".to_string()));
        }

        let reduction_level = u32::from_le_bytes(bytes[0..4].try_into()?);
        let reserved = u32::from_le_bytes(bytes[4..8].try_into()?);
        let data_offset = u64::from_le_bytes(bytes[8..16].try_into()?);
        let index_offset = u64::from_le_bytes(bytes[16..24].try_into()?);

        Ok(ZoomHeader {
            reduction_level,
            reserved,
            data_offset,
            index_offset
        })
    }
}

impl ZoomHeader {
    /// This function only works if its in the context of iterating through the zoom headers
    /// in the order they appear in the bigwig file
    pub fn matches(&self, reduction_level: f32) -> bool {
        (self.reduction_level as f32) < reduction_level
    }
}
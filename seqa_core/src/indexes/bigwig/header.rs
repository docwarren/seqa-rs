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

#[derive(Debug, Error)]
pub enum BigwigHeaderError {
    #[error("Failed to parse BigwigHeader: {0}")]
    HeaderError(String),

    #[error("Parsing error: {0}")]
    ParseError(#[from] TryFromSliceError),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigwigHeader {
	pub magic: String,  // 4 bytes
	pub version: u16,
	pub zoom_levels: u16,
	pub chromosome_tree_offset: u64,
	pub full_data_offset: u64,
	pub full_index_offset: u64,
	pub field_count: u16,
	pub defined_field_count: u16,
	pub auto_sql_offset: u64,
	pub total_summary_offset: u64,
	pub uncompress_buf_size: u32,
	pub reserved: u64,
}

impl BigwigHeader {
	pub fn new() -> Self {
		BigwigHeader {
			magic: String::from("BIGW"),
			version: 0,
			zoom_levels: 0,
			chromosome_tree_offset: 0,
			full_data_offset: 0,
			full_index_offset: 0,
			field_count: 0,
			defined_field_count: 0,
			auto_sql_offset: 0,
			total_summary_offset: 0,
			uncompress_buf_size: 0,
			reserved: 0,
		}
	}

	pub fn from_bytes(bytes: &[u8]) -> Result<Self, BigwigHeaderError> {
		if bytes.len() < 64 {
			return Err(BigwigHeaderError::HeaderError("Not enough bytes for a complete header".to_string()));
		}

		let magic = String::from_utf8_lossy(&bytes[0..4]).to_string();
		let version = u16::from_le_bytes(bytes[4..6].try_into()?);
		let zoom_levels = u16::from_le_bytes(bytes[6..8].try_into()?);
		let chromosome_tree_offset = u64::from_le_bytes(bytes[8..16].try_into()?);
		let full_data_offset = u64::from_le_bytes(bytes[16..24].try_into()?);
		let full_index_offset = u64::from_le_bytes(bytes[24..32].try_into()?);
		let field_count = u16::from_le_bytes(bytes[32..34].try_into()?);
		let defined_field_count = u16::from_le_bytes(bytes[34..36].try_into()?);
		let auto_sql_offset = u64::from_le_bytes(bytes[36..44].try_into()?);
		let total_summary_offset = u64::from_le_bytes(bytes[44..52].try_into()?);
		let uncompress_buf_size = u32::from_le_bytes(bytes[52..56].try_into()?);
		let reserved = u64::from_le_bytes(bytes[56..64].try_into()?);

		// Note: reserved field may be non-zero in some BigBed files
		// The spec says it should be 0, but we don't enforce this

		Ok(BigwigHeader {
			magic,
			version,
			zoom_levels,
			chromosome_tree_offset,
			full_data_offset,
			full_index_offset,
			field_count,
			defined_field_count,
			auto_sql_offset,
			total_summary_offset,
			uncompress_buf_size,
			reserved,
		})
	}

	pub fn is_compressed(&self) -> bool {
		self.uncompress_buf_size != 0
	}
}
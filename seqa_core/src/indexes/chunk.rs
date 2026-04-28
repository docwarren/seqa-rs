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

use super::{constants::MAX_BLOCK_SIZE, virtual_offset::VirtualOffset};
use serde::{Deserialize, Serialize};
use core::ops::Range;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChunkError {
    #[error("Parsing error: {0}")]
    ParsingError(#[from] core::array::TryFromSliceError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub bin_number: u32,
    pub begin_vp: VirtualOffset,
    pub end_vp: VirtualOffset
}

impl Chunk {
    pub fn new(bin_number: u32, begin_vp: VirtualOffset, end_vp: VirtualOffset) -> Self {
        Chunk {
            bin_number,
            begin_vp,
            end_vp
        }
    }

    pub fn to_range(&self) -> Range<u64> {
        let first_block = self.begin_vp.block_offset;
        let last_block = self.end_vp.block_offset;
        Range {
            start: first_block, 
            end: last_block + MAX_BLOCK_SIZE
        }
    }

    pub fn from_bytes(bytes: &[u8], bin_number: u32) -> Result<Self, ChunkError> {

        let cnk_beg = u64::from_le_bytes(bytes[..8].try_into()?);
        let cnk_end = u64::from_le_bytes(bytes[8..16].try_into()?);

        // Create TabixChunk and add to TabixBin
        let chunk = Chunk::new(
            bin_number,
            VirtualOffset::new(cnk_beg),
            VirtualOffset::new(cnk_end),
        );

        Ok(chunk)
    }
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        Chunk {
            bin_number: self.bin_number,
            begin_vp: self.begin_vp.clone(),
            end_vp: self.end_vp.clone()
        }
    }
}

impl Ord for Chunk {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.begin_vp.virtual_pointer.cmp(&other.begin_vp.virtual_pointer)
    }
}
impl PartialOrd for Chunk {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Chunk {
    fn eq(&self, other: &Self) -> bool {
        self.begin_vp.virtual_pointer == other.begin_vp.virtual_pointer
    }
}

impl Copy for Chunk {}
impl Eq for Chunk {}
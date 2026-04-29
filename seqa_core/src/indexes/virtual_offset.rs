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

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualOffset {
    pub virtual_pointer: u64,
    pub block_offset: u64,
    pub decompressed_offset: u64
}

impl VirtualOffset {
    pub fn new(virtual_pointer: u64) -> Self {
        let (block_offset, decompressed_offset) = (virtual_pointer >> 16, virtual_pointer & 0xFFFF);
        VirtualOffset {
            virtual_pointer,
            block_offset,
            decompressed_offset
        }
    }

    pub fn split(&self) -> (u64, u64) {
        let c_offset = self.virtual_pointer >> 16;
        let d_offset = self.virtual_pointer & 0xFFFF;
        (c_offset, d_offset)
    }
}

impl Clone for VirtualOffset {
    fn clone(&self) -> Self {
        VirtualOffset {
            virtual_pointer: self.virtual_pointer,
            block_offset: self.block_offset,
            decompressed_offset: self.decompressed_offset
        }
    }
}

impl Ord for VirtualOffset {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.virtual_pointer.cmp(&other.virtual_pointer)
    }
}

impl PartialOrd for VirtualOffset {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for VirtualOffset {
    fn eq(&self, other: &Self) -> bool {
        self.virtual_pointer == other.virtual_pointer
    }
}

impl Eq for VirtualOffset {}

impl Copy for VirtualOffset {}
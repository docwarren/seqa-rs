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

use crate::indexes::bigwig::chr_tree::BigwigChrTree;
use crate::indexes::bigwig::r_tree::RTree;
use crate::models::bigwig::zoom_data::ZoomData;

pub struct ZoomLevel {
    pub zoom_count: u32,
    pub zoom_data: Vec<ZoomData>,
    pub zoom_index: RTree,
}

impl ZoomLevel {

    pub fn from_decompressed_bytes(bytes: &[u8], chr_tree: &BigwigChrTree) -> Option<Self> {

        let mut zoom_data = Vec::new();
        let mut offset = 4;

        while offset + ZoomData::SIZE <= bytes.len() {
            if let Ok(data) = ZoomData::from_bytes(&bytes[offset..offset + ZoomData::SIZE], chr_tree) {
                zoom_data.push(data);
            }
            offset += ZoomData::SIZE;
        }

        Some(ZoomLevel {
            zoom_count: zoom_data.len() as u32,
            zoom_data,
            zoom_index: RTree::new(), // Placeholder, should be initialized properly
        })
    }
}

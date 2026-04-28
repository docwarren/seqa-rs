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

pub struct WigSectionHeader {
    pub chrom_id: u32,
    pub chrom_start: u32,
    pub chrom_end: u32,
    pub item_step: u32,
    pub item_span: u32,
    pub item_type: u8,
    pub reserved: u8,
    pub  item_count: u32
}

impl WigSectionHeader {
    pub fn new() -> Self {
        WigSectionHeader {
            chrom_id: 0,
            chrom_start: 0,
            chrom_end: 0,
            item_step: 0,
            item_span: 0,
            item_type: 0,
            reserved: 0,
            item_count: 0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 24 {
            return Err("Not enough bytes for a complete section header".into());
        }

        let chrom_id = u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| format!("Invalid Wig Section Header"))?);
        let chrom_start = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| format!("Invalid Wig Section Header"))?);
        let chrom_end = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| format!("Invalid Wig Section Header"))?);
        let item_step = u32::from_le_bytes(bytes[12..16].try_into().map_err(|_| format!("Invalid Wig Section Header"))?);
        let item_span = u32::from_le_bytes(bytes[16..20].try_into().map_err(|_| format!("Invalid Wig Section Header"))?);
        let item_type = bytes[20];
        let reserved = bytes[21];
        let item_count = u32::from_le_bytes(bytes[22..26].try_into().map_err(|_| format!("Invalid Wig Section Header"))?);

        Ok(WigSectionHeader {
            chrom_id,
            chrom_start,
            chrom_end,
            item_step,
            item_span,
            item_type,
            reserved,
            item_count,
        })
    }
}
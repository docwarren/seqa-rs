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

use std::fmt::{Display, Formatter};

use crate::indexes::bigwig::{chr_tree::BigwigChrTree, r_tree::overlaps::Overlaps};

/// Binary BED data record from BigBed file
/// Format: chromId (4), chromStart (4), chromEnd (4), rest (zero-terminated string)
#[derive(Debug, Clone)]
pub struct BigbedData {
    pub chrom_id: u32,
    pub chrom_name: String,
    pub chrom_start: u32,
    pub chrom_end: u32,
    pub rest: String,         // Tab-separated fields past the first three
    pub size: usize,          // Total byte size of this record
}

impl BigbedData {
    pub const HEADER_SIZE: usize = 12; // chromId + chromStart + chromEnd

    pub const COLUMNS: [&str; 3] = ["chromosome", "begin", "end"];

    pub fn new() -> Self {
        BigbedData {
            chrom_id: 0,
            chrom_name: String::new(),
            chrom_start: 0,
            chrom_end: 0,
            rest: String::new(),
            size: 0,
        }
    }

    /// Parse a BigBed record from bytes
    /// Records are: chromId (4), chromStart (4), chromEnd (4), rest (zero-terminated string)
    pub fn from_bytes(bytes: &[u8], chr_tree: &BigwigChrTree) -> Result<Self, String> {
        if bytes.len() < Self::HEADER_SIZE {
            return Err("Not enough bytes for a complete BigbedData record".into());
        }

        let chrom_id = u32::from_le_bytes(
            bytes[0..4]
                .try_into()
                .map_err(|e| format!("Invalid chrom_id: {}", e))?,
        );
        let chrom_start = u32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|e| format!("Invalid chrom_start: {}", e))?,
        );
        let chrom_end = u32::from_le_bytes(
            bytes[8..12]
                .try_into()
                .map_err(|e| format!("Invalid chrom_end: {}", e))?,
        );

        // Find the null terminator for the rest string
        let rest_bytes = &bytes[Self::HEADER_SIZE..];
        let null_pos = rest_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(rest_bytes.len());

        let rest = String::from_utf8_lossy(&rest_bytes[..null_pos]).to_string();

        // Total size: header + rest string + null terminator (if present)
        let size = Self::HEADER_SIZE + null_pos + 1;

        let chrom_name = chr_tree
            .get_chromosome_name(chrom_id)
            .unwrap_or_else(|| format!("chr{}", chrom_id));

        Ok(BigbedData {
            chrom_id,
            chrom_name,
            chrom_start,
            chrom_end,
            rest,
            size,
        })
    }

    /// Get field count (3 base fields + extra fields in rest)
    pub fn field_count(&self) -> usize {
        if self.rest.is_empty() {
            3
        } else {
            3 + self.rest.split('\t').count()
        }
    }
}

impl Display for BigbedData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.rest.is_empty() {
            write!(f, "{}\t{}\t{}", self.chrom_name, self.chrom_start, self.chrom_end)
        } else {
            write!(
                f,
                "{}\t{}\t{}\t{}",
                self.chrom_name, self.chrom_start, self.chrom_end, self.rest
            )
        }
    }
}

impl Overlaps for BigbedData {
    fn overlaps(&self, chrom_id1: u32, chrom_id2: u32, start: u32, end: u32) -> bool {
        ((chrom_id2 > self.chrom_id) || (chrom_id2 == self.chrom_id && end > self.chrom_start))
            && ((chrom_id1 < self.chrom_id) || (chrom_id1 == self.chrom_id && start < self.chrom_end))
    }
}

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
use crate::{indexes::bigwig::{chr_tree::BigwigChrTree, r_tree::overlaps::Overlaps}};

#[derive(Debug)]
pub struct ZoomData {
    pub chrom_id: u32,
    pub chrom_name: String,
    pub chrom_start: u32,
    pub chrom_end: u32,
    pub valid_count: u32,
    pub min: f32,
    pub max: f32,
    pub sum: f32,
    pub sum_squares: f32,
}

impl ZoomData {

    pub const SIZE: usize = 32; // Size in bytes of a ZoomData record

    pub const COLUMNS: [&str; 8] = [
        "chromosome",
        "begin",
        "end",
        "valid_count",
        "min",
        "max",
        "sum",
        "sum_squares",
    ];

    pub fn new() -> Self {
        ZoomData {
            chrom_id: 0,
            chrom_name: String::new(),
            chrom_start: 0,
            chrom_end: 0,
            valid_count: 0,
            min: 0.0,
            max: 0.0,
            sum: 0.0,
            sum_squares: 0.0,
        }
    }

    pub fn from_bytes(bytes: &[u8], chr_tree: &BigwigChrTree) -> Result<Self, String> {
        if bytes.len() < 32 {
            return Err("Not enough bytes for a complete ZoomData".into());
        }

        let chrom_id = u32::from_le_bytes(bytes[0..4].try_into().map_err(|e| format!("Invalid chrom_id: {}", e))?);
        let chrom_start = u32::from_le_bytes(bytes[4..8].try_into().map_err(|e| format!("Invalid chrom_start: {}", e))?);
        let chrom_end = u32::from_le_bytes(bytes[8..12].try_into().map_err(|e| format!("Invalid chrom_end: {}", e))?);
        let valid_count = u32::from_le_bytes(bytes[12..16].try_into().map_err(|e| format!("Invalid valid_count: {}", e))?);
        let min = f32::from_le_bytes(bytes[16..20].try_into().map_err(|e| format!("Invalid min: {}", e))?);
        let max = f32::from_le_bytes(bytes[20..24].try_into().map_err(|e| format!("Invalid max: {}", e))?);
        let sum = f32::from_le_bytes(bytes[24..28].try_into().map_err(|e| format!("Invalid sum: {}", e))?);
        let sum_squares = f32::from_le_bytes(bytes[28..32].try_into().map_err(|e| format!("Invalid sum_squares: {}", e))?);
        let chrom_name = chr_tree.get_chromosome_name(chrom_id).ok_or(format!("Chromosome ID {} not found in ChrTree", chrom_id))?;

        Ok(ZoomData {
            chrom_id,
            chrom_name,
            chrom_start,
            chrom_end,
            valid_count,
            min,
            max,
            sum,
            sum_squares,
        })
    }
}

impl Display for ZoomData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
               self.chrom_name,
               self.chrom_start,
               self.chrom_end,
               self.valid_count,
               self.min,
               self.max,
               self.sum,
               self.sum_squares)
    }
}

impl Overlaps for ZoomData {
    fn overlaps(&self, chrom_id1: u32, chrom_id2: u32,start: u32, end: u32) -> bool {
        ((chrom_id2 > self.chrom_id) || (chrom_id2 == self.chrom_id && end >= self.chrom_start)) &&
        ((chrom_id1 < self.chrom_id) || (chrom_id1 == self.chrom_id && start <= self.chrom_end))
    }
}

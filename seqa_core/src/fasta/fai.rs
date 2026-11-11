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

use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;
use crate::coordinate::Coordinate;
use crate::stores::StoreService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contig {
    pub name: String,
    pub length: u64,
    pub bases_per_line: u32,
    pub bytes_per_line: u32,
    pub offset: u64,
    pub qual_offset: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FaiIndex {
    pub contigs: HashMap<String, Contig>,
}

impl FaiIndex {
    pub fn new() -> Self {
        FaiIndex {
            contigs: HashMap::new(),
        }
    }

    pub async fn from_file(
        store_service: &StoreService,
        idx_path: &str,
    ) -> Result<Self, SearchError> {
        let bytes =
            store_service
                .get_object(idx_path)
                .await
                .map_err(|e| SearchError::ReadError {
                    path: idx_path.to_string(),
                    source: e,
                })?;

        Ok(FaiIndex::from_bytes(bytes)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ParsingError> {
        let mut fai_index = FaiIndex::new();
        let lines = String::from_utf8(bytes)?;

        for line in lines.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue; // Skip empty lines and comments
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 5 {
                continue; // Invalid line format
            }
            let contig = Contig {
                name: parts[0].to_string(),
                length: parts[1].parse().unwrap_or(0),
                offset: parts[2].parse().unwrap_or(0),
                bases_per_line: parts[3].parse().unwrap_or(0),
                bytes_per_line: parts[4].parse().unwrap_or(0),
                qual_offset: if parts.len() > 5 {
                    parts[5].parse().unwrap_or(0)
                } else {
                    0 // Default value if not provided
                },
            };
            fai_index.contigs.insert(contig.name.clone(), contig);
        }
        Ok(fai_index)
    }

    /// Returns the byte range in the FASTA file that holds the sequence for each coordinate.
    ///
    /// Coordinates are 1-based closed.  Each range starts at the first base and ends one
    /// byte after the last base, so it includes any newlines in between.
    ///
    /// # Errors
    ///
    /// Returns [`SearchError::InvalidRequest`] if a contig is not in the index, or if
    /// `begin` is 0 or `end` is past the end of the contig.
    pub fn get_offsets(&self, coordinates: &[Coordinate]) -> Result<Vec<Range<u64>>, SearchError> {
        let mut ranges = vec![];
        coordinates.iter().for_each(|coord| {
            let chr = coord.get_chromosome();
            let begin = coord.get_begin();
            let end = coord.get_end();
            if let Some(contig) = self.find_contig(chr) {
                if !FaiIndex::valid_coordinate(begin, end, contig) {
                    return;
                }
                let start = Self::get_offset(begin, contig);
                let end = Self::get_offset(end, contig) + 1;
                ranges.push(start..end);
            }
        });
        Ok(ranges)
    }

    fn find_contig(&self, chr: &str) -> Option<&Contig> {
        crate::genome::chromosome_aliases(chr)
            .iter()
            .find_map(|chr| self.contigs.get(chr))
    }

    fn valid_coordinate(begin: u32, end: u32, contig: &Contig) -> bool {
        begin >= 1 && end <= contig.length as u32
    }

    pub fn get_offset(position: u32, contig: &Contig) -> u64 {
        let line_number = if position == 1 {
            0
        } else {
            (position as u64 - 1) / contig.bases_per_line as u64
        };
        let line_offset = line_number * contig.bytes_per_line as u64;
        let base_offset = if position == 1 {
            0
        } else {
            (position as u64 - 1) % contig.bases_per_line as u64
        };
        contig.offset + line_offset + base_offset
    }
}

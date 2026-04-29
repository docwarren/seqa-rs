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
use std::fmt::Display;

use crate::traits::feature::Feature;

/// A single record from a BED (Browser Extensible Data) file.
///
/// Supports the full 12-column BED specification plus any extra columns beyond
/// column 12.  Columns 4–12 are optional; absent columns are represented as `None`.
///
/// Coordinates are **0-based half-open** `[begin, end)` as per the BED specification.
#[derive(Debug, Serialize, Deserialize)]
pub struct BedLine {
    /// Column 1 — chromosome/contig name.
    pub chromosome: String,
    /// Column 2 — 0-based start position of the feature.
    pub begin: u32,
    /// Column 3 — exclusive end position of the feature.
    pub end: u32,
    /// Column 4 — feature name/label (optional).
    pub name: Option<String>,
    /// Column 5 — score in the range 0–1000 (optional).
    pub score: Option<u16>,
    /// Column 6 — strand (`"+"` or `"-"`) (optional).
    pub strand: Option<String>,
    /// Column 7 — thickStart: start of the "thick" drawing region (optional).
    pub thick_begin: Option<u32>,
    /// Column 8 — thickEnd: end of the "thick" drawing region (optional).
    pub thick_end: Option<u32>,
    /// Column 9 — itemRgb: RGB color string, e.g. `"255,0,0"` (optional).
    pub item_rgb: Option<String>,
    /// Column 10 — number of blocks (exons) in the feature (optional).
    pub block_count: Option<u32>,
    /// Column 11 — comma-separated list of block sizes (optional).
    pub block_sizes: Option<Vec<u32>>,
    /// Column 12 — comma-separated list of block start positions relative to `begin` (optional).
    pub block_begins: Option<Vec<u32>>,
    /// Any fields beyond the standard 12 BED columns, joined by tabs.
    pub extra_fields: Option<String>,
}

impl BedLine {

    pub const COLUMNS: [&str; 12] = [
        "chromosome",
        "begin",
        "end",
        "name",
        "score",
        "strand",
        "thick_begin",
        "thick_end",
        "item_rgb",
        "block_count",
        "block_sizes",
        "block_begins"
    ];

    /// Parses a single tab-delimited BED line into a [`BedLine`].
    ///
    /// At minimum the line must contain three fields (chrom, begin, end).
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when a numeric field cannot be parsed.
    pub fn from_line(line: String) -> Result<BedLine, String> {
        let tokens = line.split('\t').collect::<Vec<&str>>();
        let chromosome = tokens[0].to_string();

        let begin = match tokens[1]
            .parse::<u32>()
        {
            Ok(pos) => pos,
            Err(_) => {
                return Err(format!("Invalid begin position in BED line: {}", tokens[1]));
            }
        };

        let end = match tokens[2]
            .parse::<u32>()
        {
            Ok(pos) => pos,
            Err(_) => {
                return Err(format!("Invalid end position in BED line: {}", tokens[2]));
            }
        };

        let name = if tokens.len() > 3 {
            Some(tokens[3].to_string())
        } else {
            None
        };

        let score = if tokens.len() > 4 {
            match tokens[4].parse::<u16>() {
                Ok(score) => Some(score),
                Err(_) => return Err(format!("Invalid score in BED line: {}", tokens[4])),
            }
        } else {
            None
        };

        let strand = if tokens.len() > 5 {
            Some(tokens[5].to_string())
        } else {
            None
        };

        let thick_begin = if tokens.len() > 6 {
            match tokens[6].parse::<u32>() {
                Ok(thick_begin) => Some(thick_begin),
                Err(_) => return Err(format!("Invalid thick_begin in BED line: {}", tokens[6])),
            }
        } else {
            None
        };

        let thick_end = if tokens.len() > 7 {
            match tokens[7].parse::<u32>() {
                Ok(thick_end) => Some(thick_end),
                Err(_) => return Err(format!("Invalid thick_end in BED line: {}", tokens[7])),
            }
        } else {
            None
        };
        let item_rgb = if tokens.len() > 8 {
            Some(tokens[8].to_string())
        } else {
            None
        };

        let block_count = if tokens.len() > 9 {
            match tokens[9].parse::<u32>() {
                Ok(block_count) => Some(block_count),
                Err(_) => return Err(format!("Invalid block_count in BED line: {}", tokens[9])),
            }
        } else {
            None
        };

        let block_sizes = if tokens.len() > 10 {
            match tokens[10]
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<u32>())
                .collect()
            {
                Ok(block_sizes) => Some(block_sizes),
                Err(_) => return Err(format!("Invalid block_sizes in BED line: {}", tokens[10])),
            }
        } else {
            None
        };

        let block_begins = if tokens.len() > 11 {
             match tokens[11]
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<u32>())
                .collect()
            {
                Ok(block_begins) => Some(block_begins),
                Err(_) => return Err(format!("Invalid block_begins in BED line: {}", tokens[11])),
            }
        } else {
            None
        };

        // Capture any extra fields beyond the standard 12 BED columns
        let extra_fields = if tokens.len() > 12 {
            Some(tokens[12..].join("\t"))
        } else {
            None
        };

        Ok(BedLine {
            chromosome,
            begin,
            end,
            name,
            score,
            strand,
            thick_begin,
            thick_end,
            item_rgb,
            block_count,
            block_sizes,
            block_begins,
            extra_fields,
        })
    }
}

impl Display for BedLine {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fields = Vec::new();

        if let Some(name) = &self.name {
            fields.push(name.clone());
        }
        if let Some(score) = self.score {
            fields.push(score.to_string());
        }
        if let Some(strand) = &self.strand {
            fields.push(strand.clone());
        }
        if let Some(thick_begin) = self.thick_begin {
            fields.push(thick_begin.to_string());
        }
        if let Some(thick_end) = self.thick_end {
            fields.push(thick_end.to_string());
        }
        if let Some(item_rgb) = &self.item_rgb {
            fields.push(item_rgb.clone());
        }
        if let Some(block_count) = self.block_count {
            fields.push(block_count.to_string());
        }
        if let Some(block_sizes) = &self.block_sizes {
            // BED spec requires trailing comma for block_sizes
            fields.push(format!("{},", block_sizes.iter().map(|b| b.to_string()).collect::<Vec<String>>().join(",")));
        }
        if let Some(block_begins) = &self.block_begins {
            // BED spec requires trailing comma for block_begins
            fields.push(format!("{},", block_begins.iter().map(|b| b.to_string()).collect::<Vec<String>>().join(",")));
        }
        if let Some(extra) = &self.extra_fields {
            fields.push(extra.clone());
        }
        write!(f, "{}\t{}\t{}\t{}", self.chromosome, self.begin, self.end, fields.join("\t"))
    }
}

impl Feature for BedLine {

    fn get_chromosome(&self) -> String {
        self.chromosome.clone()
    }

    fn get_begin(&self) -> u32 {
        self.begin
    }

    fn get_end(&self) -> u32 {
        self.end
    }

    fn get_length(&self) -> u32 {
        self.end - self.begin
    }

    fn get_id(&self) -> String {
        if let Some(name) = &self.name {
            name.clone()
        } else {
            format!("{}:{}-{}", self.chromosome, self.begin, self.end)
        }
    }

    fn coordinate_system(&self) -> super::coordinates::CoordinateSystem {
        super::coordinates::CoordinateSystem::ZeroBasedHalfOpen
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_init_from_string() {
        let test_line1 = "chr12\t0\t133851895\tdup\t0\t.\t0\t133851895\t0,0,255\t1\t133851895\t0".to_string();
        let test_line2 = "chr12\t111270\t146659\t12p13.33x3\t0\t.\t111270\t146659\t0,0,255\t1\t35389\t0".to_string();

        // BED spec requires trailing commas for block_sizes and block_begins
        let expected_1 = "chr12\t0\t133851895\tdup\t0\t.\t0\t133851895\t0,0,255\t1\t133851895,\t0,".to_string();
        let expected_2 = "chr12\t111270\t146659\t12p13.33x3\t0\t.\t111270\t146659\t0,0,255\t1\t35389,\t0,".to_string();

        let bed1 = BedLine::from_line(test_line1).expect("Failed to parse BED line 1");
        assert_eq!(format!("{}", bed1), expected_1);

        let bed2 = BedLine::from_line(test_line2).expect("Failed to parse BED line 2");
        assert_eq!(format!("{}", bed2), expected_2);
    }
}

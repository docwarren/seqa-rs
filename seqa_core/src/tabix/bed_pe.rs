use crate::api::parsing_error::ParsingError;
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
use crate::models::coordinate_system::CoordinateSystem;
use crate::parsing_utils::split_str;
use crate::traits::feature::Feature;
use crate::{coordinate::Coordinate, traits::from_bytes::FromBytes};
use bstr::ByteSlice;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// A single record from a BED (Browser Extensible Data) file.
///
/// Supports the full 12-column BED specification plus any extra columns beyond
/// column 12.  Columns 4–12 are optional; absent columns are represented as `None`.
///
/// Coordinates are **0-based half-open** `[begin, end)` as per the BED specification.
#[derive(Debug, Serialize, Deserialize)]
pub struct BedpeLine {
    /// Column 1 — chromosome/contig name.
    pub chromosome1: String,
    /// Column 2 — 0-based start position of the feature.
    pub begin1: u32,
    /// Column 3 — exclusive end position of the feature.
    pub end1: u32,
    /// Column 4 — feature name/label (optional).
    pub chromosome2: String,
    /// Column 5 — score in the range 0–1000 (optional).
    pub begin2: u32,
    /// Column 6 — strand (`"+"` or `"-"`) (optional).
    pub end2: u32,
    /// Column 7 — thickStart: start of the "thick" drawing region (optional).
    pub name: Option<String>,
    /// Column 8 — thickEnd: end of the "thick" drawing region (optional).
    pub score: Option<f32>,
    /// Column 9 — itemRgb: RGB color string, e.g. `"255,0,0"` (optional).
    pub strand1: Option<String>,
    /// Column 10 — number of blocks (exons) in the feature (optional).
    pub strand2: Option<String>,
    /// Any fields beyond the standard 12 BED columns, joined by tabs.
    pub extra_fields: Option<String>,
}

impl BedpeLine {
    pub const COLUMNS: [&str; 10] = [
        "chromosome1",
        "begin1",
        "end1",
        "chromosome2",
        "begin2",
        "end2",
        "name",
        "score",
        "strand1",
        "strand2",
    ];
}

impl FromBytes for BedpeLine {
    type Error = ParsingError;

    fn from_bytes(bytes: &[u8]) -> Result<(BedpeLine, usize), Self::Error> {
        let tokens = split_str(bytes, "\t", Some(6))?;
        let chromosome1 = tokens[0].to_str()?;
        let begin1 = tokens[1].to_str()?.parse::<u32>()?;
        let end1 = tokens[2].to_str()?.parse::<u32>()?;
        let chromosome2 = tokens[3].to_str()?;
        let begin2 = tokens[4].to_str()?.parse::<u32>()?;
        let end2 = tokens[5].to_str()?.parse::<u32>()?;

        let name = if tokens.len() > 6 {
            tokens[6].to_str().ok()
        } else {
            None
        };

        let score = if tokens.len() > 7 {
            tokens[7].to_str()?.parse::<f32>().ok()
        } else {
            None
        };

        let strand1 = if tokens.len() > 8 {
            tokens[8].to_str().ok()
        } else {
            None
        };

        let strand2 = if tokens.len() > 9 {
            tokens[9].to_str().ok()
        } else {
            None
        };

        let extra_fields = if tokens.len() > 10 {
            let mut fields = String::new();
            let extra_tokens = &tokens[10..];
            for (i, byt) in extra_tokens.iter().enumerate() {
                fields.push_str(byt.to_str()?);
                if i < extra_tokens.len() - 1 {
                    fields.push_str("\t")
                }
            }

            Some(fields)
        } else {
            None
        };

        Ok((
            BedpeLine {
                chromosome1: chromosome1.to_string(),
                begin1,
                end1,
                chromosome2: chromosome2.to_string(),
                begin2,
                end2,
                name: name.map(|n| n.to_string()),
                score,
                strand1: strand1.map(|s| s.to_string()),
                strand2: strand2.map(|s| s.to_string()),
                extra_fields,
            },
            bytes.len(),
        ))
    }
}

impl Display for BedpeLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fields = vec![
            self.chromosome1.clone(),
            self.begin1.to_string(),
            self.end1.to_string(),
            self.chromosome2.clone(),
            self.begin2.to_string(),
            self.end2.to_string(),
        ];

        if let Some(name) = &self.name {
            fields.push(name.clone());
        }
        if let Some(score) = self.score {
            fields.push(score.to_string());
        }
        if let Some(strand1) = &self.strand1 {
            fields.push(strand1.clone());
        }
        if let Some(strand2) = &self.strand2 {
            fields.push(strand2.clone());
        }
        if let Some(extra) = &self.extra_fields {
            fields.push(extra.clone());
        }
        write!(f, "{}", fields.join("\t"))
    }
}

impl Feature for BedpeLine {
    fn get_id(&self) -> String {
        self.name.clone().unwrap_or(format!(
            "{}:{}-{};{}:{}-{}",
            self.chromosome1, self.begin1, self.end1, self.chromosome2, self.begin2, self.end2
        ))
    }

    fn coordinate(&self) -> Coordinate {
        Coordinate {
            chromosome: self.chromosome1.clone(),
            begin: self.begin1,
            end: self.end1,
            system: CoordinateSystem::ZeroBasedHalfOpen,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_init_from_bytes() {
        let test1 = b"chr1\t10\t20\tchr5\t50\t60\ta1\t30\t+\t-\t0\t1";
        let test2 = b"chr9\t30\t40\tchr9\t80\t90\ta2\t100\t+\t-\t2\t1";
        let expected1 = "chr1\t10\t20\tchr5\t50\t60\ta1\t30\t+\t-\t0\t1".to_string();
        let expected2 = "chr9\t30\t40\tchr9\t80\t90\ta2\t100\t+\t-\t2\t1".to_string();

        let bed1 = BedpeLine::from_bytes(test1)
            .expect("Failed to parse BED line 1")
            .0;
        assert_eq!(format!("{}", bed1), expected1);

        let bed2 = BedpeLine::from_bytes(test2)
            .expect("Failed to parse BED line 2")
            .0;
        assert_eq!(format!("{}", bed2), expected2);
    }

    #[test]
    fn it_should_display_without_optional_fields() {
        let test1 = b"chr1\t10\t20\tchr5\t50\t60";
        let test2 = b"chr1\t10\t20\tchr5\t50\t60\ta1\t30";
        let expected1 = "chr1\t10\t20\tchr5\t50\t60".to_string();
        let expected2 = "chr1\t10\t20\tchr5\t50\t60\ta1\t30".to_string();

        let bed1 = BedpeLine::from_bytes(test1)
            .expect("Failed to parse BEDPE line 1")
            .0;
        assert_eq!(format!("{}", bed1), expected1);

        let bed2 = BedpeLine::from_bytes(test2)
            .expect("Failed to parse BEDPE line 2")
            .0;
        assert_eq!(format!("{}", bed2), expected2);
    }
}

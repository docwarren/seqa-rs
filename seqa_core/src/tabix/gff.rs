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
use crate::coordinate::Coordinate;
use crate::models::coordinate_system::CoordinateSystem;
use crate::traits::from_bytes::FromBytes;
use bstr::ByteSlice;

pub struct GffLine {
    pub chromosome: String,
    pub source: String,
    pub feature: String,
    pub begin: u32,
    pub end: u32,
    pub score: Option<u16>,
    pub strand: Option<String>,
    pub frame: Option<u8>,
    pub group: Option<String>,
}

impl GffLine {
    pub const COLUMNS: [&str; 9] = [
        "chromosome",
        "source",
        "feature",
        "begin",
        "end",
        "score",
        "strand",
        "frame",
        "group",
    ];
}

impl FromBytes for GffLine {
    type Error = ParsingError;

    fn from_bytes(bytes: &[u8]) -> Result<(GffLine, usize), Self::Error> {
        let tokens = bytes.to_str()?.split("\t").collect::<Vec<&str>>();
        let chromosome = tokens[0];
        let source = tokens[1];
        let feature = tokens[2];
        let begin = tokens[3].parse::<u32>()?;
        let end = tokens[4].parse::<u32>()?;

        let score = if tokens[5] == "." {
            None
        } else {
            Some(tokens[5].parse::<u16>()?)
        };

        let strand = if tokens[6] == "." {
            None
        } else {
            Some(tokens[6])
        };

        let frame = if tokens[7] == "." {
            None
        } else {
            Some(tokens[7].parse::<u8>()?)
        };

        let group = if tokens[8] == "." {
            None
        } else {
            Some(tokens[8])
        };

        Ok((
            GffLine {
                chromosome: chromosome.to_string(),
                source: source.to_string(),
                feature: feature.to_string(),
                begin,
                end,
                score,
                strand: strand.map(|s| s.to_string()),
                frame,
                group: group.map(|g| g.to_string()),
            },
            bytes.len(),
        ))
    }
}

impl std::fmt::Display for GffLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.chromosome,
            self.source,
            self.feature,
            self.begin,
            self.end,
            self.score
                .as_ref()
                .map_or(".".to_string(), |s| s.to_string()),
            self.strand.as_ref().map_or(".".to_string(), |s| s.clone()),
            self.frame
                .as_ref()
                .map_or(".".to_string(), |f| f.to_string()),
            self.group.as_ref().map_or(".".to_string(), |g| g.clone())
        )
    }
}

impl crate::traits::feature::Feature for GffLine {
    fn get_id(&self) -> String {
        format!("{}:{}-{}", self.chromosome, self.begin, self.end)
    }

    fn coordinate(&self) -> Coordinate {
        Coordinate {
            chromosome: self.chromosome.clone(),
            begin: self.begin,
            end: self.end,
            system: CoordinateSystem::OneBasedClosed,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_init_from_string() {
        let test_line1 = b"chr22\tTeleGene\tenhancer\t10000000\t10001000\t500\t+\t.\ttouch1";
        let test_line2 = b"chr22\tTeleGene\tpromoter\t10020000\t10025000\t800\t-\t.\ttouch2";

        let expected_1 =
            "chr22\tTeleGene\tenhancer\t10000000\t10001000\t500\t+\t.\ttouch1".to_string();
        let expected_2 =
            "chr22\tTeleGene\tpromoter\t10020000\t10025000\t800\t-\t.\ttouch2".to_string();

        let gff1 = GffLine::from_bytes(test_line1)
            .expect("Failed to parse GFF line 1")
            .0;
        assert_eq!(format!("{}", gff1), expected_1);

        let gff2 = GffLine::from_bytes(test_line2)
            .expect("Failed to parse GFF line 2")
            .0;
        assert_eq!(format!("{}", gff2), expected_2);
    }

    #[test]
    fn it_should_display_without_optional_fields() {
        let test_line = b"chr22\tTeleGene\tenhancer\t10000000\t10001000\t.\t.\t.\t.";
        let expected = "chr22\tTeleGene\tenhancer\t10000000\t10001000\t.\t.\t.\t.".to_string();

        let gff = GffLine::from_bytes(test_line)
            .expect("Failed to parse GFF line")
            .0;
        assert_eq!(format!("{}", gff), expected);
    }
}

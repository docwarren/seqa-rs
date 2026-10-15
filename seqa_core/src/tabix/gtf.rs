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
use crate::parsing_utils::split_str;
use crate::traits::from_bytes::FromBytes;
use bstr::ByteSlice;

pub struct GtfLine {
    pub chromosome: String,
    pub source: String,
    pub feature: String,
    pub begin: u32,
    pub end: u32,
    pub score: Option<u16>,
    pub strand: Option<String>,
    pub frame: Option<u8>,
    pub attributes: Vec<(String, String)>,
}

impl GtfLine {

    pub const COLUMNS: [&str; 9] = [
        "chromosome",
        "source",
        "feature",
        "begin",
        "end",
        "score",
        "strand",
        "frame",
        "attributes"
    ];
}

impl FromBytes for GtfLine {
    type Error = ParsingError;

    fn from_bytes(bytes: &[u8]) -> Result<(GtfLine, usize), Self::Error> {
        let tokens = split_str(bytes, "\t", Some(9))?;
        let chromosome = tokens[0].to_str()?;
        let source = tokens[1].to_str()?;
        let feature = tokens[2].to_str()?;
        let begin = tokens[3].to_str()?.parse::<u32>()?;
        let end = tokens[4].to_str()?.parse::<u32>()?;

        let score = if tokens[5] == b"." {
            None
        } else {
            Some(tokens[5].to_str()?.parse::<u16>()?)
        };

        let strand = if tokens[6] == b"." {
            None
        } else {
            Some(tokens[6].to_str()?)
        };

        let frame = if tokens[7] == b"." {
            None
        } else {
            Some(tokens[7].to_str()?.parse::<u8>()?)
        };

        // Attributes are `key value` pairs separated by `;`
        let mut attributes = vec![];
        if tokens[8] != b"." {
            let attribute_tokens = split_str(tokens[8], ";", None)?;
            for attribute in attribute_tokens {
                let parts = split_str(attribute.trim(), " ", None)?;
                if parts.len() == 2 {
                    attributes.push((
                        parts[0].trim().to_str()?.to_string(),
                        parts[1].trim().to_str()?.to_string(),
                    ));
                }
            }
        }

        Ok((
            GtfLine {
                chromosome: chromosome.to_string(),
                source: source.to_string(),
                feature: feature.to_string(),
                begin,
                end,
                score,
                strand: strand.map(|s| s.to_string()),
                frame,
                attributes,
            },
            bytes.len(),
        ))
    }
}

impl std::fmt::Display for GtfLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.chromosome,
            self.source,
            self.feature,
            self.begin,
            self.end,
            self.score.as_ref().map_or(".".to_string(), |s| s.to_string()),
            self.strand.as_ref().map_or(".".to_string(), |s| s.clone()),
            self.frame.as_ref().map_or(".".to_string(), |f| f.to_string()),
            if self.attributes.is_empty() {
                ".".to_string()
            } else {
                self.attributes
                    .iter()
                    .map(|(k, v)| format!("{} {}", k, v))
                    .collect::<Vec<String>>()
                    .join(";")
            }
        )
    }
}

impl crate::traits::feature::Feature for GtfLine {

    fn get_id(&self) -> String {
        format!("{}:{}-{}", self.chromosome, self.begin, self.end)
    }

    fn coordinate(&self) -> Coordinate {
        Coordinate {
            chromosome: self.chromosome.clone(),
            begin: self.begin,
            end: self.end,
            system: CoordinateSystem::OneBasedClosed
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_init_gtf_from_string() {
        let test_line1 = b"chr12\trefGene\texon\t14477\t14944\t.\t-\t.\tgene_id \"WASH8P\";transcript_id \"NR_130745\";exon_number \"1\";exon_id \"NR_130745.1\";gene_name \"WASH8P\"";
        let test_line2 = b"chr12\trefGene\texon\t18373\t18484\t.\t-\t.\tgene_id \"WASH8P\";transcript_id \"NR_130745\";exon_number \"9\";exon_id \"NR_130745.9\";gene_name \"WASH8P\"";

        let expected_1 = test_line1.to_str().unwrap().to_string();
        let expected_2 = test_line2.to_str().unwrap().to_string();

        let gtf1 = GtfLine::from_bytes(test_line1)
            .expect("Failed to parse GTF line 1")
            .0;
        assert_eq!(format!("{}", gtf1), expected_1);

        let gtf2 = GtfLine::from_bytes(test_line2)
            .expect("Failed to parse GTF line 2")
            .0;
        assert_eq!(format!("{}", gtf2), expected_2);
    }

    #[test]
    fn it_should_display_without_optional_fields() {
        let test_line = b"chr12\trefGene\texon\t14477\t14944\t.\t.\t.\t.";
        let expected = "chr12\trefGene\texon\t14477\t14944\t.\t.\t.\t.".to_string();

        let gtf = GtfLine::from_bytes(test_line)
            .expect("Failed to parse GTF line")
            .0;
        assert_eq!(format!("{}", gtf), expected);
    }
}

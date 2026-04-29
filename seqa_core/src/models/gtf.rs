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

    pub fn from_line(line: String) -> Result<GtfLine, String> {
        let tokens = line.split('\t').collect::<Vec<&str>>();
        if tokens.len() < 9 {
            return Err(format!("Invalid GTF line: {}", line));
        }

        let chromosome = tokens[0].to_string();
        let source = tokens[1].to_string();
        let feature = tokens[2].to_string();

        let begin = match tokens[3].parse::<u32>() {
            Ok(b) => b - 1,
            Err(_) => return Err(format!("Invalid begin position in GTF line: {}", line)),
        };

        let end = match tokens[4].parse::<u32>() {
            Ok(e) => e,
            Err(_) => return Err(format!("Invalid end position in GTF line: {}", line)),
        };

        let score = if tokens[5] == "." {
            None
        } else {
            match tokens[5].parse::<u16>() {
                Ok(s) => Some(s),
                Err(_) => return Err(format!("Invalid score in GTF line: {}", line)),
            }
        };

        let strand = if tokens[6] == "." {
            None
        } else {
            Some(tokens[6].to_string())
        };

        let frame = if tokens[7] == "." {
            None
        } else {
            match tokens[7].parse::<u8>() {
                Ok(f) => Some(f),
                Err(_) => return Err(format!("Invalid frame in GTF line: {}", line)),
            }
        };

        let attributes = if tokens.len() > 8 && tokens[8] != "." {
            tokens[8]
                .split(';')
                .filter_map(|attr| {
                    let attr = attr.trim();
                    let parts: Vec<&str> = attr.split(' ').collect();
                    if parts.len() == 2 {
                        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(GtfLine {
            chromosome,
            source,
            feature,
            begin,
            end,
            score,
            strand,
            frame,
            attributes,
        })
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
            self.begin + 1, // Convert back to position for display
            self.end,
            self.score.as_ref().map_or(".".to_string(), |s| s.to_string()),
            self.strand.as_ref().map_or(".".to_string(), |s| s.clone()),
            self.frame.as_ref().map_or(".".to_string(), |f| f.to_string()),
            self.attributes
                .iter()
                .map(|(k, v)| format!("{} {}", k, v))
                .collect::<Vec<String>>()
                .join(";")
        )
    }
}

impl crate::traits::feature::Feature for GtfLine {
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
        format!("{}:{}-{}", self.chromosome, self.begin, self.end)
    }

    fn coordinate_system(&self) -> super::coordinates::CoordinateSystem {
        super::coordinates::CoordinateSystem::OneBasedClosed
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_init_gtf_from_string() {
        let test_line1 = "chr12\trefGene\texon\t14477\t14944\t.\t-\t.\tgene_id \"WASH8P\";transcript_id \"NR_130745\";exon_number \"1\";exon_id \"NR_130745.1\";gene_name \"WASH8P\"".to_string();
        let test_line2 = "chr12\trefGene\texon\t18373\t18484\t.\t-\t.\tgene_id \"WASH8P\";transcript_id \"NR_130745\";exon_number \"9\";exon_id \"NR_130745.9\";gene_name \"WASH8P\"".to_string();

        let expected_1 = test_line1.clone();
        let expected_2 = test_line2.clone();

        let bed1 = GtfLine::from_line(test_line1).expect("Failed to parse GTF line 1");
        assert_eq!(format!("{}", bed1), expected_1);

        let bed2 = GtfLine::from_line(test_line2).expect("Failed to parse GTF line 2");
        assert_eq!(format!("{}", bed2), expected_2);
    }
}

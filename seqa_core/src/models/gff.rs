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
        "group"
    ];

    pub fn from_line(line: String) -> Result<GffLine, String> {
        let tokens = line.split('\t').collect::<Vec<&str>>();
        if tokens.len() < 9 {
            return Err(format!("Invalid GFF line: {}", line));
        }

        let chromosome = tokens[0].to_string();
        let source = tokens[1].to_string();
        let feature = tokens[2].to_string();

        let begin = match tokens[3]
            .parse::<u32>()
        {
            Ok(b) => b,
            Err(_) => return Err(format!("Invalid begin position in GFF line: {}", line)),
        };

        let end = match tokens[4]
            .parse::<u32>()
        {
            Ok(e) => e,
            Err(_) => return Err(format!("Invalid end position in GFF line: {}", line)),
        };

        let score = if tokens[5] == "." {
            None
        } else {
            match tokens[5].parse::<u16>() {
                Ok(s) => Some(s),
                Err(_) => return Err(format!("Invalid score in GFF line: {}", line)),
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
                Err(_) => return Err(format!("Invalid frame in GFF line: {}", line)),
            }
        };

        let group = if tokens.len() > 8 && tokens[8] != "." {
            Some(tokens[8].to_string())
        } else {
            None
        };

        Ok(GffLine {
            chromosome,
            source,
            feature,
            begin,
            end,
            score,
            strand,
            frame,
            group,
        })
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
            self.score.as_ref().map_or(".".to_string(), |s| s.to_string()),
            self.strand.as_ref().map_or(".".to_string(), |s| s.clone()),
            self.frame.as_ref().map_or(".".to_string(), |f| f.to_string()),
            self.group.as_ref().map_or(".".to_string(), |g| g.clone())
        )
    }
}

impl crate::traits::feature::Feature for GffLine {
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
    fn it_should_init_from_string() {
        let test_line1 = "chr22\tTeleGene\tenhancer\t10000000\t10001000\t500\t+\t.\ttouch1".to_string();
        let test_line2 = "chr22\tTeleGene\tpromoter\t10020000\t10025000\t800\t-\t.\ttouch2".to_string();

        let expected_1 = test_line1.clone();
        let expected_2 = test_line2.clone();

        let bed1 = GffLine::from_line(test_line1).expect("Failed to parse GFF line 1");
        assert_eq!(format!("{}", bed1), expected_1);

        let bed2 = GffLine::from_line(test_line2).expect("Failed to parse GFF line 2");
        assert_eq!(format!("{}", bed2), expected_2);
    }
}

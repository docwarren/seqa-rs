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

use std::fmt::Display;

use crate::traits::feature::Feature;

pub struct BedGraphLine {
    pub chrom: String,
    pub begin: u32,
    pub end: u32,
    pub value: f64,
}

impl BedGraphLine {

    pub const COLUMNS: [&str; 4] = [
        "chromosome",
        "begin",
        "end",
        "value"
    ];

    pub fn from_line(line: String) -> Result<BedGraphLine, String> {
        let tokens = line.split('\t').collect::<Vec<&str>>();
        if tokens.len() < 4 {
            return Err(format!("Invalid BEDGRAPH line: {}", line));
        }

        let chrom = tokens[0].to_string();
        let begin = match tokens[1].parse::<u32>() {
            Ok(b) => b,
            Err(e) => return Err(format!("Invalid begin position in BEDGRAPH line: {}: {}", line, e)),
        };
        let end = match tokens[2].parse::<u32>() {
            Ok(e) => e,
            Err(e) => return Err(format!("Invalid end position in BEDGRAPH line: {}: {}", line, e)),
        };
        let value = match tokens[3].parse::<f64>() {
            Ok(v) => v,
            Err(e) => return Err(format!("Invalid value in BEDGRAPH line: {}: {}", line, e)),
        };

        Ok(BedGraphLine {
            chrom,
            begin,
            end,
            value,
        })
    }
}

impl Display for BedGraphLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}\t{}",
            self.chrom, self.begin, self.end, self.value
        )
    }
}

impl Feature for BedGraphLine {

    fn get_chromosome(&self) -> String {
        self.chrom.clone()
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
        format!("{}:{}-{}", self.chrom, self.begin, self.end)
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
        let test_line1 = "chr8\t3096256\t3096446\t0.0223076923076923".to_string();
        let test_line2 = "chr8\t3099824\t3100068\t0.0223076923076923".to_string();

        let expected_1 = test_line1.clone();
        let expected_2 = test_line2.clone();

        let bed1 = BedGraphLine::from_line(test_line1).expect("Failed to parse BEDGRAPH line 1");
        assert_eq!(format!("{}", bed1), expected_1);

        let bed2 = BedGraphLine::from_line(test_line2).expect("Failed to parse BEDGRAPH line 2");
        assert_eq!(format!("{}", bed2), expected_2);
    }
}

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

use bstr::ByteSlice;

use crate::api::parsing_error::ParsingError;
use crate::coordinate::Coordinate;
use crate::models::coordinate_system::CoordinateSystem;
use crate::traits::feature::Feature;
use crate::traits::from_bytes::FromBytes;
use std::fmt::Display;

pub struct BedGraphLine {
    pub chrom: String,
    pub begin: u32,
    pub end: u32,
    pub value: f64,
}

impl FromBytes for BedGraphLine {
    type Error = ParsingError;

    fn from_bytes(bytes: &[u8]) -> Result<(BedGraphLine, usize), Self::Error> {
        let tokens: Vec<&str> = bytes.to_str()?.split("\t").collect();
        let chrom = tokens[0];
        let begin = tokens[1].parse::<u32>()?;
        let end = tokens[2].parse::<u32>()?;
        let value = tokens[3].parse::<f64>()?;

        Ok((
            BedGraphLine {
                chrom: chrom.to_string(),
                begin,
                end,
                value,
            },
            bytes.len(),
        ))
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

    fn coordinate(&self) -> Coordinate {
        Coordinate {
            chromosome: self.chrom.clone(),
            begin: self.begin,
            end: self.end,
            system: CoordinateSystem::ZeroBasedHalfOpen,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_init_from_string() {
        let test_line1 = b"chr8\t3096256\t3096446\t0.0223076923076923";
        let test_line2 = b"chr8\t3099824\t3100068\t0.0223076923076923";

        let expected_1 = "chr8\t3096256\t3096446\t0.0223076923076923".to_string();
        let expected_2 = "chr8\t3099824\t3100068\t0.0223076923076923".to_string();

        let bed1 = BedGraphLine::from_bytes(test_line1)
            .expect("Failed to parse BEDGRAPH line 1")
            .0;
        assert_eq!(format!("{}", bed1), expected_1);

        let bed2 = BedGraphLine::from_bytes(test_line2)
            .expect("Failed to parse BEDGRAPH line 2")
            .0;
        assert_eq!(format!("{}", bed2), expected_2);
    }
}

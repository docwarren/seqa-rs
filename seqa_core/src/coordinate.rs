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
use serde::{Deserialize, Serialize};
use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;
use crate::models::coordinate_system::CoordinateSystem;
use crate::utils::parse_coordinates;

/// Test each coordinate to ensure it doesnt request too much data
pub fn check_coords_for_length(coords: &Vec<Coordinate>, len: usize) -> Result<(), SearchError> {
    for coord in coords {
        if coord.get_length() as usize > len {
            return Err(SearchError::InvalidRequest {
                requested: format!("{}", coord),
                reason: "Maximum query size for fasta files 250MBases".to_string()
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinate {
    chromosome: String,
    begin: u32,
    end: u32,
    system: CoordinateSystem,
}

impl Display for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}\t{}\t{}", self.chromosome, self.begin, self.end)
    }
}

/// A genomic interval with a chromosome, begin, and end position.
///
/// All record types (`VcfLine`, `BedLine`, `GffLine`, `GtfLine`, `BedGraphLine`) and
/// [`crate::api::search_options::SearchOptions`] implement this trait so that interval
/// arithmetic (e.g. overlap detection) works uniformly across formats.
///
/// Implementors must report their native [`CoordinateSystem`].  The provided
/// [`Feature::to_canonical`] and [`Feature::overlaps`] methods use that information to
/// convert everything to **0-based half-open** coordinates before comparing.
impl Coordinate {
    pub fn new(coord_str: &str, system: CoordinateSystem) -> Result<Coordinate, ParsingError> {
        let result = parse_coordinates(coord_str)
            .map_err(|e| ParsingError::FormatError(format!("{e}")))?;
        Ok(Coordinate {
            chromosome: result.0,
            begin: result.1,
            end: result.2,
            system
        })
    }
    /// Returns the chromosome/contig name (e.g. `"chr1"` or `"1"`).
    pub fn get_chromosome(&self) -> &str {
        &self.chromosome
    }
    /// Returns the begin position in the feature's native coordinate system.
    pub fn get_begin(&self) -> u32 {
        self.begin
    }
    /// Returns the end position in the feature's native coordinate system.
    pub fn get_end(&self) -> u32 {
        self.end
    }
    /// Returns the length of the feature in bases.
    pub fn get_length(&self) -> u32 {
        match self.system {
            CoordinateSystem::OneBasedClosed => (self.end - self.begin) + 1,
            CoordinateSystem::ZeroBasedHalfOpen => self.end - self.begin
        }
    }
    /// Returns the coordinate system used by this coordinate.
    pub fn coordinate_system(&self) -> CoordinateSystem {
        self.system
    }

    /// Converts begin/end to **0-based half-open** canonical coordinates.
    pub fn to_canonical(&self) -> (u32, u32) {
        self.system.to_canonical(self.begin, self.end)
    }

    /// Returns `true` if this feature overlaps `other` on the same chromosome.
    ///
    /// Both features are converted to canonical coordinates before comparison,
    /// so VCF/GFF (1-based) and BED/BAM (0-based) features can be compared directly.
    pub fn overlaps(&self, other: Coordinate) -> bool {
        let (begin, end) = self.to_canonical();
        let (other_begin, other_end) = other.to_canonical();
        let chr1 = self.get_chromosome().replace("chr", "");
        let chr2 = other.get_chromosome().replace("chr", "");
        chr1 == chr2 && begin < other_end && end > other_begin
    }
}

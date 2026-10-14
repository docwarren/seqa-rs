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
use crate::models::coordinate_system::CoordinateSystem;
use crate::utils::parse_coordinates;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Checks that no coordinate is longer than `len` bases.
///
/// # Errors
///
/// Returns [`SearchError::InvalidRequest`] for the first coordinate that is too long.
pub fn check_coords_for_length(coords: &[Coordinate], len: usize) -> Result<(), SearchError> {
    for coord in coords {
        if coord.get_length() as usize > len {
            return Err(SearchError::InvalidRequest {
                requested: format!("{}", coord),
                reason: format!(
                    "Maximum query size exceeded: coords length: {}, max length: {}",
                    coord.get_length(),
                    len
                ),
            });
        }
    }
    Ok(())
}

/// A genomic interval: a chromosome plus begin and end positions in a given
/// [`CoordinateSystem`].
///
/// `begin` and `end` are stored as given.  [`Coordinate::to_canonical`] and
/// [`Coordinate::overlaps`] convert to **0-based half-open** first, so coordinates in
/// different systems can be compared directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinate {
    /// Chromosome/contig name (e.g. `"chr1"` or `"1"`).
    pub chromosome: String,
    /// Begin position in `system`.
    pub begin: u32,
    /// End position in `system`.
    pub end: u32,
    /// The coordinate system `begin` and `end` are expressed in.
    pub system: CoordinateSystem,
}

/// Formats as `chromosome\tbegin\tend` using the native positions.
impl Display for Coordinate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}\t{}\t{}", self.chromosome, self.begin, self.end)
    }
}

impl Coordinate {
    /// Parses a coordinate string such as `"chr1:1000-2000"` and tags it with `system`.
    ///
    /// See [`crate::utils::parse_coordinates`] for the accepted formats.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError::FormatError`] if the string cannot be parsed.
    pub fn new(coord_str: &str, system: CoordinateSystem) -> Result<Coordinate, ParsingError> {
        let result =
            parse_coordinates(coord_str).map_err(|e| ParsingError::FormatError(format!("{e}")))?;
        Ok(Coordinate {
            chromosome: result.0,
            begin: result.1,
            end: result.2,
            system,
        })
    }
    /// Returns the chromosome/contig name (e.g. `"chr1"` or `"1"`).
    pub fn get_chromosome(&self) -> &str {
        &self.chromosome
    }
    /// Returns the begin position in this coordinate's own system.
    pub fn get_begin(&self) -> u32 {
        self.begin
    }
    /// Returns the end position in this coordinate's own system.
    pub fn get_end(&self) -> u32 {
        self.end
    }
    /// Returns the length of the interval in bases.
    pub fn get_length(&self) -> u32 {
        match self.system {
            CoordinateSystem::OneBasedClosed => (self.end - self.begin) + 1,
            CoordinateSystem::ZeroBasedHalfOpen => self.end - self.begin,
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

    /// Returns `true` if this interval overlaps `other` on the same chromosome.
    ///
    /// Both intervals are converted to canonical coordinates before comparison,
    /// so VCF/GFF (1-based) and BED/BAM (0-based) coordinates can be compared directly.
    /// Chromosome names are compared with any `"chr"` prefix removed, so `"chr1"`
    /// matches `"1"`.
    pub fn overlaps(&self, other: &Coordinate) -> bool {
        let (begin, end) = self.to_canonical();
        let (other_begin, other_end) = other.to_canonical();
        let chr1 = self.get_chromosome().replace("chr", "");
        let chr2 = other.get_chromosome().replace("chr", "");
        chr1 == chr2 && begin < other_end && end > other_begin
    }

    /// Returns `true` if the positions of this interval overlap `other`, without
    /// comparing chromosomes.
    ///
    /// For callers that have already matched the chromosome some other way, e.g. BAM
    /// reads, whose coordinates carry a reference index rather than a name.
    pub fn overlaps_ignore_chromosome(&self, other: &Coordinate) -> bool {
        let (begin, end) = self.to_canonical();
        let (other_begin, other_end) = other.to_canonical();
        begin < other_end && end > other_begin
    }
}

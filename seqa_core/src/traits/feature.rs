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

use crate::models::coordinates::CoordinateSystem;

/// A genomic interval with a chromosome, begin, and end position.
///
/// All record types (`VcfLine`, `BedLine`, `GffLine`, `GtfLine`, `BedGraphLine`) and
/// [`crate::api::search_options::SearchOptions`] implement this trait so that interval
/// arithmetic (e.g. overlap detection) works uniformly across formats.
///
/// Implementors must report their native [`CoordinateSystem`].  The provided
/// [`Feature::to_canonical`] and [`Feature::overlaps`] methods use that information to
/// convert everything to **0-based half-open** coordinates before comparing.
pub trait Feature: std::fmt::Display {
    /// Returns the chromosome/contig name (e.g. `"chr1"` or `"1"`).
    fn get_chromosome(&self) -> String;
    /// Returns the begin position in the feature's native coordinate system.
    fn get_begin(&self) -> u32;
    /// Returns the end position in the feature's native coordinate system.
    fn get_end(&self) -> u32;
    /// Returns the length of the feature in bases.
    fn get_length(&self) -> u32;
    /// Returns a unique identifier for the feature (e.g. rsID for VCF, name for BED).
    fn get_id(&self) -> String;
    /// Returns the coordinate system used by this feature type.
    fn coordinate_system(&self) -> CoordinateSystem;

    /// Converts begin/end to **0-based half-open** canonical coordinates.
    fn to_canonical(&self) -> (u32, u32) {
        self.coordinate_system().to_canonical(self.get_begin(), self.get_end())
    }

    /// Returns `true` if this feature overlaps `other` on the same chromosome.
    ///
    /// Both features are converted to canonical coordinates before comparison,
    /// so VCF/GFF (1-based) and BED/BAM (0-based) features can be compared directly.
    fn overlaps(&self, other: &dyn Feature) -> bool {
        let (begin, end) = self.to_canonical();
        let (other_begin, other_end) = other.to_canonical();
        self.get_chromosome() == other.get_chromosome() && begin < other_end && end > other_begin
    }
}

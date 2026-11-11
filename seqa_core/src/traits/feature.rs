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

use crate::coordinate::Coordinate;

/// A record parsed from a genomic file that has a position on the genome.
///
/// Implemented by the record types (`VcfLine`, `BedLine`, `BedGraphLine`, `GffLine`,
/// `GtfLine` and BAM `Read`) so that overlap detection works the same way across
/// formats: compare [`Feature::coordinate`] against a query with
/// [`Coordinate::overlaps`].
///
/// `Display` must write the record back out in its native text format; the tabix
/// search uses it to produce result lines.
pub trait Feature: std::fmt::Display {
    /// Returns the feature's position, tagged with its format's native coordinate system.
    fn coordinate(&self) -> Coordinate;
}

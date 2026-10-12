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

use crate::models::coordinate_system::CoordinateSystem;

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
    /// Returns a unique identifier for the feature (e.g. rsID for VCF, name for BED).
    fn get_id(&self) -> String;
    /// Returns the coordinate system used by this feature type.
    fn coordinate_system(&self) -> CoordinateSystem;
}

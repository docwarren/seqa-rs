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

use serde::{ Serialize, Deserialize };

/// The coordinate numbering convention used by a genomic file format.
///
/// The library's internal canonical format is **0-based half-open** `[begin, end)`.
///
/// | Format | System |
/// |--------|--------|
/// | BED, BAM, BigWig, BigBed | [`ZeroBasedHalfOpen`](CoordinateSystem::ZeroBasedHalfOpen) |
/// | VCF, GFF, GTF | [`OneBasedClosed`](CoordinateSystem::OneBasedClosed) |
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CoordinateSystem {
    /// 0-based, half-open interval `[begin, end)` — used by BED, BAM, BigWig.
    ZeroBasedHalfOpen,
    /// 1-based, closed interval `[begin, end]` — used by VCF, GFF, GTF.
    OneBasedClosed,
}

impl CoordinateSystem {
    /// Converts a `(begin, end)` pair from this coordinate system to canonical
    /// 0-based half-open form.
    ///
    /// For [`OneBasedClosed`](CoordinateSystem::OneBasedClosed), `begin` is decremented by 1.
    pub fn to_canonical(&self, begin: u32, end: u32) -> (u32, u32) {
        match self {
            CoordinateSystem::ZeroBasedHalfOpen => (begin, end),
            CoordinateSystem::OneBasedClosed => (begin - 1, end),
        }
    }

    /// Converts a canonical 0-based half-open `(begin, end)` pair back to this
    /// coordinate system.
    ///
    /// For [`OneBasedClosed`](CoordinateSystem::OneBasedClosed), `begin` is incremented by 1.
    pub fn from_canonical(&self, begin: u32, end: u32) -> (u32, u32) {
        match self {
            CoordinateSystem::ZeroBasedHalfOpen => (begin, end),
            CoordinateSystem::OneBasedClosed => (begin + 1, end),
        }
    }
}

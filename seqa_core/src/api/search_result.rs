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

use crate::bam::bai::BaiIndex;
use crate::bigwig::index::bigwig_index::BigwigIndex;
use crate::fasta::fai::FaiIndex;
use crate::tabix::tabix_index::TabixIndex;
use crate::traits::to_lines::ToLines;

/// The result of a genomic region search.
///
/// Each format-specific search function populates the relevant index/header fields
/// so that callers can cache the parsed index across subsequent queries without
/// re-downloading it.  The actual data lines are always in [`SearchResult::lines`].
#[derive(Default)]
pub struct SearchResult {
    /// Parsed BigWig index; present after a [`crate::bigwig::bigwig_search::bigwig_search`] call.
    pub bigwig_index: Option<BigwigIndex>,
    /// Parsed BigBed index; present after a [`crate::bigwig::bigbed_search::bigbed_search`] call.
    /// BigBed shares the same index structure as BigWig.
    pub bigbed_index: Option<BigwigIndex>,
    /// Parsed tabix index; present after a [`crate::tabix::tabix_search::tabix_search`] call.
    pub tabix_index: Option<TabixIndex>,
    /// Parsed FASTA index (`.fai`); present after a [`crate::fasta::fasta_search::fasta_search`] call.
    pub fasta_index: Option<FaiIndex>,
    /// Parsed BAM index (`.bai`); present after a [`crate::bam::bam_search::bam_search`] call.
    pub bam_index: Option<BaiIndex>,
    /// BAM file header; present after a [`crate::bam::bam_search::bam_search`] call.
    pub header: Option<Box<dyn ToLines>>,
    /// Array of Coordinate Strings corresponding to the array of result strings
    pub coordinates: Vec<String>,
    /// Query result lines in the native text format of the file (SAM, VCF, BED, etc.).
    pub lines: Vec<Vec<String>>,
}

impl SearchResult {
    /// Creates an empty `SearchResult` with all optional fields set to `None`.
    pub fn new() -> SearchResult {
        SearchResult {
            bigwig_index: None,
            bigbed_index: None,
            tabix_index: None,
            bam_index: None,
            fasta_index: None,
            header: None,
            coordinates: Vec::new(),
            lines: Vec::new(),
        }
    }
}

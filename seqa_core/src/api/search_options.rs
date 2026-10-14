use super::output_format::OutputFormat;
use crate::api::parsing_error::ParsingError;
use crate::bam::bai::BaiIndex;
use crate::bam::header::header::BamHeader;
use crate::bigwig::index::bigwig_index::BigwigIndex;
use crate::coordinate::Coordinate;
use crate::fasta::fai::FaiIndex;
use crate::models::coordinate_system::CoordinateSystem;
use crate::tabix::tabix::Tabix;
use crate::tabix::tabix_header::TabixHeader;
use crate::utils::get_output_format;
use serde::{Deserialize, Serialize};
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

/// Specifies the format of the CIGAR string in BAM output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CigarFormat {
    /// Standard CIGAR string as stored in the BAM file (e.g., "10M1D5M")
    #[default]
    Standard,
    /// Merged CIGAR with mismatch and deletion bases included (e.g., "10M1DA5M1XG")
    Merged,
}

/// Configuration for a genomic region search.
///
/// Build a `SearchOptions` with [`SearchOptions::new`] and the setter methods, or set
/// fields directly.  Pass the finished value to [`crate::api::search::search_features`],
/// or to a format-specific search function such as [`crate::bam::bam_search::bam_search`]
/// or [`crate::tabix::tabix_search::tabix_search`].
///
/// # Example
///
/// ```rust
/// use seqa_core::api::search_options::SearchOptions;
///
/// let opts = SearchOptions::new("s3://my-bucket/sample.bam", &vec!["chr1".to_string()])
///     .unwrap()
///     .set_index_path("s3://my-bucket/sample.bam.bai")
///     .set_output_format("bam");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// URI of the genomic data file (local `file://`, `s3://`, `az://`, `gs://`, `http(s)://`).
    pub file_path: String,
    /// URI of the companion index file.  Set to `"-"` for self-indexed formats
    /// (BigWig, BigBed).
    pub index_path: String,
    /// Regions to search, in the order results are returned.  Usually one, but may be many.
    pub coordinates: Vec<Coordinate>,
    /// Optional reference genome build (e.g. `"hg38"`, `"hg19"`), stored in lowercase.
    /// Not currently used by the searches: whole-chromosome coordinates always resolve to
    /// the longest known chromosome length across all supported builds.
    pub genome: Option<String>,
    /// File format of [`SearchOptions::file_path`].
    pub output_format: OutputFormat,
    /// Whether to include the file header lines in [`crate::api::search_result::SearchResult::lines`].
    /// Defaults to `true`.
    pub include_header: bool,
    /// When `true`, return only header lines and skip data records.
    /// Defaults to `false`.
    pub header_only: bool,
    /// Controls how CIGAR strings are rendered in BAM/SAM output.
    pub cigar_format: CigarFormat,
    /// Pre-parsed BigWig index; supply to avoid re-downloading on repeated queries.
    pub bigwig_index: Option<BigwigIndex>,
    /// Pre-parsed BigBed index; supply to avoid re-downloading on repeated queries.
    /// BigBed shares the same index structure as BigWig.
    pub bigbed_index: Option<BigwigIndex>,
    /// Pre-parsed BAM index; supply to avoid re-downloading on repeated queries.
    pub bam_index: Option<BaiIndex>,
    /// Pre-parsed BAM header; supply to avoid re-downloading on repeated queries.
    pub bam_header: Option<BamHeader>,
    /// Pre-parsed tabix index; supply to avoid re-downloading on repeated queries.
    pub tabix_index: Option<Tabix>,
    /// Pre-parsed tabix header; supply to avoid re-downloading on repeated queries.
    pub tabix_header: Option<TabixHeader>,
    /// Pre-parsed FASTA index; supply to avoid re-downloading on repeated queries.
    pub fasta_index: Option<FaiIndex>,
    /// When `true`, skip reading from and writing to the local index cache.
    /// The index will always be fetched from the remote source.
    /// Defaults to `false`.
    pub no_cache: bool,
}

impl SearchOptions {
    /// Creates a `SearchOptions` for `path` and one or more coordinate strings.
    ///
    /// - `path` is normalised with [`crate::utils::format_file_path`], so local paths become
    ///   `file://` URIs.
    /// - `index_path` is inferred from the file extension (e.g. `.bam` → `.bam.bai`, `"-"` for
    ///   BigWig/BigBed), falling back to `path` itself if the extension is not recognised.
    /// - `output_format` is inferred from the file extension, falling back to
    ///   [`OutputFormat::STRING`].
    /// - Each coordinate string is parsed with [`crate::utils::parse_coordinates`] as
    ///   **1-based closed**.
    ///
    /// `include_header` defaults to `true`; `header_only` and `no_cache` default to `false`.
    ///
    /// # Errors
    ///
    /// Returns [`ParsingError::FormatError`] if any coordinate string cannot be parsed.
    pub fn new(path: &str, coord_strings: &[String]) -> Result<Self, ParsingError> {
        let system = CoordinateSystem::OneBasedClosed;
        let file_path = crate::utils::format_file_path(path).unwrap_or_else(|_| path.to_string());
        let index_path =
            crate::utils::get_index_path(&file_path).unwrap_or_else(|_| file_path.clone());
        let output_format = get_output_format(&file_path).unwrap_or(OutputFormat::STRING);
        let coordinates = coord_strings
            .iter()
            .map(|c| Coordinate::new(c, system))
            .collect::<Result<Vec<Coordinate>, ParsingError>>()?;

        Ok(SearchOptions {
            file_path,
            index_path,
            coordinates,
            genome: None,
            output_format,
            cigar_format: CigarFormat::Standard,
            include_header: true,
            header_only: false,
            no_cache: false,
            bigwig_index: None,
            bigbed_index: None,
            bam_header: None,
            bam_index: None,
            tabix_index: None,
            tabix_header: None,
            fasta_index: None,
        })
    }

    /// Sets the CIGAR string rendering format for BAM output.
    pub fn set_cigar_format(&mut self, cigar_format: CigarFormat) -> Self {
        self.cigar_format = cigar_format;
        self.clone()
    }

    /// Sets the URI of the companion index file.
    pub fn set_index_path(&mut self, index_path: &str) -> Self {
        self.index_path = index_path.to_string();
        self.clone()
    }

    /// Sets the output format by name (case-insensitive).
    /// Falls back to [`OutputFormat::VCF`] if the name is unrecognised.
    pub fn set_output_format(&mut self, output_format: &str) -> Self {
        let format = output_format.to_lowercase();
        self.output_format = OutputFormat::get_enum(&format).unwrap_or(OutputFormat::VCF);
        self.clone()
    }

    /// Controls whether file header lines are included in the result.
    pub fn set_include_header(&mut self, include_header: bool) -> Self {
        self.include_header = include_header;
        self.clone()
    }

    /// Sets the reference genome build (stored in lowercase).
    ///
    /// Supported values: `"hg19"`, `"hg38"`, `"grch37"`, `"grch38"`.
    /// See [`SearchOptions::genome`]: the value is stored but not currently used by the searches.
    pub fn set_genome(&mut self, genome: &str) -> Self {
        self.genome = Some(genome.to_lowercase());
        self.clone()
    }

    /// When `true`, only header lines are returned and data records are skipped.
    pub fn set_header_only(&mut self, header_only: bool) -> Self {
        self.header_only = header_only;
        self.clone()
    }

    /// When `true`, skip reading from and writing to the local index cache.
    pub fn set_no_cache(&mut self, no_cache: bool) -> Self {
        self.no_cache = no_cache;
        self.clone()
    }
}

impl Display for SearchOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.coordinates.iter() {
            write!(
                f,
                "{}:{}-{}\n",
                c.get_chromosome(),
                c.get_begin(),
                c.get_end()
            )?;
        }
        Ok(())
    }
}

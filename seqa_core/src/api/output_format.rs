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

use crate::tabix::bed::BedLine;
use crate::tabix::bed_pe::BedpeLine;
use crate::tabix::bedgraph::BedGraphLine;
use crate::tabix::gff::GffLine;
use crate::tabix::gtf::GtfLine;
use crate::tabix::vcf::VcfLine;
use crate::traits::feature::Feature;
use crate::{api::parsing_error::ParsingError, traits::from_bytes::FromBytes};
use serde::{Deserialize, Serialize};

/// The genomic file format being queried.
///
/// `OutputFormat` controls which index parser and record model are used during a search.
/// It is usually inferred automatically from the file extension via
/// [`crate::utils::get_output_format`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Binary Alignment Map — requires a companion `.bai` index.
    BAM,
    /// FASTA sequence file — requires a companion `.fai` index.
    FASTA,
    /// Variant Call Format (bgzipped + tabix-indexed `.vcf.gz`).
    VCF,
    /// Browser Extensible Data Paired End (bgzipped + tabix-indexed `.bed.gz`).
    BEDPE,
    /// Browser Extensible Data (bgzipped + tabix-indexed `.bed.gz`).
    BED,
    /// BedGraph signal track (bgzipped + tabix-indexed `.bedgraph.gz`).
    BEDGRAPH,
    /// BigWig signal track — index is embedded in the file.
    BIGWIG,
    /// BigBed annotation track — index is embedded in the file.
    BIGBED,
    /// General Feature Format v3 (bgzipped + tabix-indexed `.gff.gz` or `.gff3.gz`).
    GFF,
    /// Gene Transfer Format (bgzipped + tabix-indexed `.gtf.gz`).
    GTF,
    /// Raw string output; no format-specific parsing is applied.
    STRING,
}

impl OutputFormat {
    /// Parses a format name string (case-insensitive) into an [`OutputFormat`] variant.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when `format` does not match any known format name.
    pub(crate) fn get_enum(format: &str) -> Result<OutputFormat, String> {
        match format.to_lowercase().as_str() {
            "bam" => Ok(OutputFormat::BAM),
            "fasta" => Ok(OutputFormat::FASTA),
            "vcf" => Ok(OutputFormat::VCF),
            "bed" => Ok(OutputFormat::BED),
            "bedpe" => Ok(OutputFormat::BEDPE),
            "bedgraph" => Ok(OutputFormat::BEDGRAPH),
            "bigwig" => Ok(OutputFormat::BIGWIG),
            "bigbed" => Ok(OutputFormat::BIGBED),
            "gff" => Ok(OutputFormat::GFF),
            "gtf" => Ok(OutputFormat::GTF),
            "string" => Ok(OutputFormat::STRING),
            _ => Err(format!("Unknown output format: {}", format)),
        }
    }

    /// Returns the record-parsing function for this format.
    ///
    /// The returned function parses a single tab-delimited text line into a
    /// boxed [`crate::traits::feature::Feature`].  Only text-based tabix formats
    /// (VCF, BED, BedGraph, GFF, GTF) are supported; all other variants return a
    /// function that always errors with "Unsupported output format".
    pub(crate) fn read_from_bytes(&self, line: &[u8]) -> Result<Box<dyn Feature>, ParsingError> {
        match self {
            OutputFormat::VCF => Ok(Box::new(VcfLine::from_bytes(line)?.0)),
            OutputFormat::BED => Ok(Box::new(BedLine::from_bytes(line)?.0)),
            OutputFormat::BEDGRAPH => Ok(Box::new(BedGraphLine::from_bytes(line)?.0)),
            OutputFormat::BEDPE => Ok(Box::new(BedpeLine::from_bytes(line)?.0)),
            OutputFormat::GFF => Ok(Box::new(GffLine::from_bytes(line)?.0)),
            OutputFormat::GTF => Ok(Box::new(GtfLine::from_bytes(line)?.0)),
            _ => Err(ParsingError::FormatError(
                "Unsupported output format".to_string(),
            )),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::BAM => write!(f, "BAM"),
            OutputFormat::FASTA => write!(f, "FASTA"),
            OutputFormat::VCF => write!(f, "VCF"),
            OutputFormat::BED => write!(f, "BED"),
            OutputFormat::BEDPE => write!(f, "BEDPE"),
            OutputFormat::BEDGRAPH => write!(f, "BEDGRAPH"),
            OutputFormat::BIGWIG => write!(f, "BIGWIG"),
            OutputFormat::BIGBED => write!(f, "BIGBED"),
            OutputFormat::GFF => write!(f, "GFF"),
            OutputFormat::GTF => write!(f, "GTF"),
            OutputFormat::STRING => write!(f, "STRING"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_enum_parses_format_names() {
        assert_eq!(OutputFormat::get_enum("bam").unwrap(), OutputFormat::BAM);
        assert_eq!(OutputFormat::get_enum("VCF").unwrap(), OutputFormat::VCF);
        assert!(OutputFormat::get_enum("unknown").is_err());
    }
}

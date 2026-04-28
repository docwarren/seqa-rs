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

use serde::{ Serialize, Deserialize};

use crate::models::bedgraph::BedGraphLine;
use crate::models::vcf::VcfLine;
use crate::models::bed::BedLine;
use crate::traits::feature::Feature;
use crate::models::gff::GffLine;
use crate::models::gtf::GtfLine;

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
    /// Browser Extensible Data (bgzipped + tabix-indexed `.bed.gz`, or plain `.bed`).
    BED,
    /// BedGraph signal track (bgzipped + tabix-indexed `.bedgraph.gz`).
    BEDGRAPH,
    /// BigWig signal track — index is embedded in the file.
    BIGWIG,
    /// BigBed annotation track — index is embedded in the file.
    BIGBED,
    /// General Feature Format v3 (bgzipped + tabix-indexed `.gff.gz`).
    GFF,
    /// Gene Transfer Format (bgzipped + tabix-indexed `.gtf.gz`).
    GTF,
    /// Raw string output; no format-specific parsing is applied.
    STRING
}

impl OutputFormat {
    /// Parses a format name string (case-insensitive) into an [`OutputFormat`] variant.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when `format` does not match any known format name.
    ///
    /// # Examples
    ///
    /// ```
    /// use seqa_core::api::output_format::OutputFormat;
    ///
    /// assert_eq!(OutputFormat::from_str("bam").unwrap(), OutputFormat::BAM);
    /// assert_eq!(OutputFormat::from_str("VCF").unwrap(), OutputFormat::VCF);
    /// assert!(OutputFormat::from_str("unknown").is_err());
    /// ```
    pub fn from_str(format: &str) -> Result<OutputFormat, String> {
        match format.to_lowercase().as_str() {
            "bam" => Ok(OutputFormat::BAM),
            "fasta" => Ok(OutputFormat::FASTA),
            "vcf" => Ok(OutputFormat::VCF),
            "bed" => Ok(OutputFormat::BED),
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
    pub fn get_model(&self) -> fn(&str) -> Result<Box<dyn Feature>, String> {
        match self {
            OutputFormat::VCF => vcf_from_line,
            OutputFormat::BED => bed_from_line,
            OutputFormat::BEDGRAPH => bedgraph_from_line,
            OutputFormat::GFF => gff_from_line,
            OutputFormat::GTF => gtf_from_line,
            _ => unsupported_format,
        }
    }
}

fn vcf_from_line(line: &str) -> Result<Box<dyn Feature>, String> {
    match VcfLine::from_line(line.to_string()) {
        Ok(line) => Ok(Box::new(line)),
        Err(err) => Err(err),
    }
}

fn bed_from_line(line: &str) -> Result<Box<dyn Feature>, String> {
    match BedLine::from_line(line.to_string()) {
        Ok(line) => Ok(Box::new(line)),
        Err(err) => Err(err),
    }
}

fn bedgraph_from_line(line: &str) -> Result<Box<dyn Feature>, String> {
    match BedGraphLine::from_line(line.to_string()) {
        Ok(line) => Ok(Box::new(line)),
        Err(err) => Err(err),
    }
}

fn gff_from_line(line: &str) -> Result<Box<dyn Feature>, String> {
    match GffLine::from_line(line.to_string()) {
        Ok(line) => Ok(Box::new(line)),
        Err(err) => Err(err),
    }
}

fn gtf_from_line(line: &str) -> Result<Box<dyn Feature>, String> {
    match GtfLine::from_line(line.to_string()) {
        Ok(line) => Ok(Box::new(line)),
        Err(err) => Err(err),
    }
}

fn unsupported_format(_: &str) -> Result<Box<dyn Feature>, String> {
    Err("Unsupported output format".to_string())
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::BAM => write!(f, "BAM"),
            OutputFormat::FASTA => write!(f, "FASTA"),
            OutputFormat::VCF => write!(f, "VCF"),
            OutputFormat::BED => write!(f, "BED"),
            OutputFormat::BEDGRAPH => write!(f, "BEDGRAPH"),
            OutputFormat::BIGWIG => write!(f, "BIGWIG"),
            OutputFormat::BIGBED => write!(f, "BIGBED"),
            OutputFormat::GFF => write!(f, "GFF"),
            OutputFormat::GTF => write!(f, "GTF"),
            OutputFormat::STRING => write!(f, "STRING"),
        }
    }
}

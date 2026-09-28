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
use std::path::absolute;

use serde::{Deserialize, Serialize};

use crate::api::output_format::OutputFormat;
use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;
use crate::genome::get_longest_possible_genome;

/// Normalises a file path to a URI accepted by [`crate::stores::StoreService`].
///
/// - Cloud/HTTP URIs (`s3://`, `gs://`, `az://`, `http://`, …) are returned unchanged.
/// - Bare local paths (`/`, `./`, `../`) are resolved to an absolute path and
///   prefixed with `file://`.  An error is returned if the file does not exist.
///
/// # Errors
///
/// Returns [`UtilError`] when a local path does not exist or cannot be canonicalised.
pub fn format_file_path(file_path: &str) -> Result<String, SearchError> {
    if file_path.starts_with("/") || file_path.starts_with("./") || file_path.starts_with("../") {
        match std::fs::exists(file_path) {
            Ok(true) => (),
            Ok(false) => {
                return Err(SearchError::NotFound {
                    path: file_path.to_string(),
                });
            }
            Err(e) => {
                return Err(SearchError::AbsolutePathError {
                    path: file_path.to_string(),
                    source: e,
                });
            }
        }
        let abs_path = absolute(file_path).map_err(|e| SearchError::AbsolutePathError {
            path: file_path.to_string(),
            source: e,
        })?;
        Ok(format!("file://{}", abs_path.to_string_lossy()))
    } else {
        Ok(file_path.to_string())
    }
}

/// Parses a genomic coordinate string into `(chromosome, begin, end)`.
///
/// Accepted formats:
/// - `"chr1"` — whole chromosome; `begin` = 1, `end` = chromosome's maximum length.
/// - `"chr1:1000"` — single-base query; `end` = `begin`.
/// - `"chr1:1000-2000"` — explicit range.
/// - Commas in numbers are stripped (e.g. `"chr1:1,000-2,000"`).
///
/// # Errors
///
/// Returns [`FormatError`] when the chromosome name is not recognised or a
/// numeric field cannot be parsed.
pub fn parse_coordinates(coords: &str) -> Result<(String, u32, u32), SearchError> {
    let tokens: Vec<&str> = coords.split(':').collect();
    let chromosome = tokens[0].to_string();
    let chr_idx = crate::genome::chr_index(&chromosome).ok_or(
        SearchError::InvalidCoordinateFormat(format!("Invalid chromosome: {}.", chromosome)),
    )?;

    let (begin, end) = get_begin_end(&tokens, chr_idx).map_err(|e| {
        SearchError::InvalidCoordinateFormat(format!(
            "Could not parse coordinates {}, {}",
            coords, e
        ))
    })?;
    Ok((chromosome, begin, end))
}

fn get_begin_end(tokens: &[&str], chr_idx: usize) -> Result<(u32, u32), ParsingError> {
    let longest_genome = get_longest_possible_genome();
    if tokens.len() == 2 {
        let parts: Vec<String> = tokens[1].split('-').map(|s| s.replace(",", "")).collect();

        if parts.len() == 2 {
            let begin = parts[0].parse::<u32>()?;
            let end = parts[1].parse::<u32>()?;
            Ok((begin, end))
        } else {
            let begin = parts[0].parse::<u32>()?;
            let end = longest_genome[chr_idx];
            Ok((begin, end))
        }
    } else {
        Ok((1, longest_genome[chr_idx]))
    }
}

/// A supported genomic file format, its extensions, and how its index is found.
///
/// This table is the single source of truth for extension handling.  Both
/// [`get_output_format`] and [`get_index_path`] derive from it, so they cannot
/// drift apart as the two hand-written if-else chains they replaced did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatSpec {
    /// The format these extensions map to.
    pub format: OutputFormat,
    /// Recognised extensions, each including the leading dot.
    pub extensions: &'static [&'static str],
    /// Extension appended to the data file to locate its index, or `None` when
    /// the index is embedded in the file itself (BigWig, BigBed).
    pub index_extension: Option<&'static str>,
}

/// Every format the search engine can query.
///
/// Text-based tabix formats appear only in their bgzipped form — a plain
/// `.vcf`, `.bed` or `.gff` cannot be range-queried, so offering it would
/// produce a file that fails at load time.
pub const SUPPORTED_FORMATS: &[FormatSpec] = &[
    FormatSpec {
        format: OutputFormat::BAM,
        extensions: &[".bam"],
        index_extension: Some(".bai"),
    },
    FormatSpec {
        format: OutputFormat::FASTA,
        extensions: &[".fa", ".fasta"],
        index_extension: Some(".fai"),
    },
    FormatSpec {
        format: OutputFormat::BIGWIG,
        extensions: &[".bigwig", ".bw"],
        index_extension: None,
    },
    FormatSpec {
        format: OutputFormat::BIGBED,
        extensions: &[".bigbed", ".bb"],
        index_extension: None,
    },
    FormatSpec {
        format: OutputFormat::VCF,
        extensions: &[".vcf.gz"],
        index_extension: Some(".tbi"),
    },
    FormatSpec {
        format: OutputFormat::BED,
        extensions: &[".bed.gz"],
        index_extension: Some(".tbi"),
    },
    FormatSpec {
        format: OutputFormat::BEDGRAPH,
        extensions: &[".bedgraph.gz"],
        index_extension: Some(".tbi"),
    },
    FormatSpec {
        format: OutputFormat::GFF,
        extensions: &[".gff.gz", ".gff3.gz"],
        index_extension: Some(".tbi"),
    },
    FormatSpec {
        format: OutputFormat::GTF,
        extensions: &[".gtf.gz"],
        index_extension: Some(".tbi"),
    },
];

/// Finds the [`FormatSpec`] whose extension matches `file_path`.
///
/// Matching is longest-extension-first, so `sample.bedgraph.gz` resolves to
/// BEDGRAPH rather than being shadowed by a shorter suffix.
pub fn match_format(file_path: &str) -> Option<&'static FormatSpec> {
    let lower_path = file_path.to_ascii_lowercase();

    SUPPORTED_FORMATS
        .iter()
        .filter_map(|spec| {
            spec.extensions
                .iter()
                .filter(|extension| lower_path.ends_with(*extension))
                .map(|extension| (extension.len(), spec))
                .max_by_key(|(length, _)| *length)
        })
        .max_by_key(|(length, _)| *length)
        .map(|(_, spec)| spec)
}

/// A serialisable view of [`SUPPORTED_FORMATS`] for clients.
///
/// The frontend loads this once at startup and uses it to decide which files a
/// user may select, so it does not have to duplicate the extension table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFormat {
    pub format: String,
    pub extensions: Vec<String>,
    /// `None` when the index is embedded in the data file.
    pub index_extension: Option<String>,
}

/// Returns the supported format table in serialisable form.
pub fn supported_formats() -> Vec<FileFormat> {
    SUPPORTED_FORMATS
        .iter()
        .map(|spec| FileFormat {
            format: format!("{:?}", spec.format),
            extensions: spec.extensions.iter().map(|e| e.to_string()).collect(),
            index_extension: spec.index_extension.map(|e| e.to_string()),
        })
        .collect()
}

/// Infers the companion index URI from a genomic file URI.
///
/// Returns `"-"` for formats whose index is embedded in the file itself
/// (BigWig, BigBed).  Derived from [`SUPPORTED_FORMATS`].
///
/// # Errors
///
/// Returns [`SearchError::IndexPathError`] when the extension is not supported.
pub fn get_index_path(file_path: &str) -> Result<String, SearchError> {
    let spec = match_format(file_path).ok_or_else(|| {
        SearchError::IndexPathError("Unable to get index path from file extension".into())
    })?;

    Ok(match spec.index_extension {
        Some(extension) => format!("{}{}", file_path, extension),
        None => "-".to_string(),
    })
}

/// Infers the [`OutputFormat`] from a file URI's extension.
///
/// Derived from [`SUPPORTED_FORMATS`].
///
/// # Errors
///
/// Returns [`SearchError::PathTypeError`] when the extension does not match any
/// known format.
pub fn get_output_format(file_path: &str) -> Result<OutputFormat, SearchError> {
    match_format(file_path)
        .map(|spec| spec.format.clone())
        .ok_or_else(|| {
            SearchError::PathTypeError("Unable to determine file format from extension".into())
        })
}

#[test]
fn test_parse_coordinates() {
    let (chr, start, end) = parse_coordinates("chr1:1000-2000").unwrap();
    assert_eq!(chr, "chr1");
    assert_eq!(start, 1000);
    assert_eq!(end, 2000);
}

#[test]
fn test_get_search_options_local() {
    use crate::api::search_options::SearchOptions;
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let options = SearchOptions::new("./mock_data/NA12878.gatk.cnv.vcf.gz", "chr1:1000-2000");
    assert_eq!(
        options.file_path,
        format!("file://{}/mock_data/NA12878.gatk.cnv.vcf.gz", manifest_dir)
    );
    assert_eq!(options.chromosome, "chr1");
    assert_eq!(options.begin, 1000);
    assert_eq!(options.end, 2000);
    assert_eq!(
        options.index_path,
        format!(
            "file://{}/mock_data/NA12878.gatk.cnv.vcf.gz.tbi",
            manifest_dir
        )
    );
    assert_eq!(options.output_format, OutputFormat::VCF);

    let options = SearchOptions::new("./mock_data/NA12878.gatk.cnv.vcf.gz", "chr1");
    assert_eq!(options.chromosome, "chr1");
    assert_eq!(options.begin, 1);
    assert_eq!(options.end, 249_250_621); // Assuming chr1 length from longest_possible_genome
}

#[test]
fn test_format_file_path() {
    let path = format_file_path("./mock_data/NA12878.gatk.cnv.vcf.gz").unwrap();
    assert!(path.starts_with("file://"));
    let path = format_file_path("./fake/path.vcf.gz");
    assert!(path.is_err());

    let path = format_file_path("s3://bucket/file.vcf.gz").unwrap();
    assert_eq!(path, "s3://bucket/file.vcf.gz");
    let path = format_file_path("gs://bucket/file.vcf.gz").unwrap();
    assert_eq!(path, "gs://bucket/file.vcf.gz");
    let path = format_file_path("https://example.com/file.vcf.gz").unwrap();
    assert_eq!(path, "https://example.com/file.vcf.gz");
    let path = format_file_path("https://example.com/file.vcf.gz").unwrap();
    assert_eq!(path, "https://example.com/file.vcf.gz");
}

#[test]
fn test_get_index_path() {
    let index = get_index_path("s3://bucket/file.bam").unwrap();
    assert_eq!(index, "s3://bucket/file.bam.bai");
    let index = get_index_path("gs://bucket/file.fa").unwrap();
    assert_eq!(index, "gs://bucket/file.fa.fai");
    let index = get_index_path("https://example.com/file.vcf.gz").unwrap();
    assert_eq!(index, "https://example.com/file.vcf.gz.tbi");
    let index = get_index_path("https://example.com/file.bigwig").unwrap();
    assert_eq!(index, "-");
    let index = get_index_path("file.bed.gz").unwrap();
    assert_eq!(index, "file.bed.gz.tbi");
    // Plain `.bed` is no longer indexable: it is not bgzipped, so it cannot be
    // range-queried.  `get_output_format` always rejected it; now both agree.
    assert!(get_index_path("file.bed").is_err());
    let index = get_index_path("file.unknown");
    assert!(index.is_err());
}

#[test]
fn test_match_format_prefers_longest_extension() {
    // `.bedgraph.gz` must not be shadowed by a shorter suffix.
    assert_eq!(
        get_output_format("sample.bedgraph.gz").unwrap(),
        OutputFormat::BEDGRAPH
    );
    assert_eq!(
        get_output_format("sample.bed.gz").unwrap(),
        OutputFormat::BED
    );
}

#[test]
fn test_gff3_is_supported() {
    assert_eq!(
        get_output_format("genes.gff3.gz").unwrap(),
        OutputFormat::GFF
    );
    assert_eq!(
        get_index_path("genes.gff3.gz").unwrap(),
        "genes.gff3.gz.tbi"
    );
}

#[test]
fn test_plain_text_formats_are_rejected() {
    // Un-bgzipped tabix formats cannot be range-queried.
    for path in ["a.vcf", "a.bed", "a.gff", "a.gtf", "a.bedgraph", "a.gff3"] {
        assert!(
            get_output_format(path).is_err(),
            "{path} should be rejected"
        );
        assert!(get_index_path(path).is_err(), "{path} should have no index");
    }
}

#[test]
fn test_cram_and_csi_are_not_supported() {
    assert!(get_output_format("sample.cram").is_err());
    assert!(match_format("sample.bam.csi").is_none());
}

#[test]
fn test_embedded_index_formats() {
    for path in ["a.bigwig", "a.bw", "a.bigbed", "a.bb"] {
        assert_eq!(get_index_path(path).unwrap(), "-");
        assert!(match_format(path).unwrap().index_extension.is_none());
    }
}

#[test]
fn test_match_format_is_case_insensitive() {
    assert_eq!(get_output_format("SAMPLE.BAM").unwrap(), OutputFormat::BAM);
    assert_eq!(
        get_output_format("Sample.VCF.GZ").unwrap(),
        OutputFormat::VCF
    );
}

#[test]
fn test_supported_formats_is_serialisable_view() {
    let formats = supported_formats();
    assert_eq!(formats.len(), SUPPORTED_FORMATS.len());

    let bam = formats.iter().find(|f| f.format == "BAM").unwrap();
    assert_eq!(bam.extensions, vec![".bam"]);
    assert_eq!(bam.index_extension.as_deref(), Some(".bai"));

    let bigwig = formats.iter().find(|f| f.format == "BIGWIG").unwrap();
    assert_eq!(bigwig.index_extension, None);
}

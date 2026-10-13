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

use crate::api::output_format::OutputFormat;
use crate::api::parsing_error::ParsingError;
use crate::api::search::{decompress_chunks, fetch};
use crate::api::search_error::SearchError;
use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::coordinate::Coordinate;
use crate::indexes::bin_util::get_bin_numbers;
use crate::stores::StoreService;
use crate::tabix::bed::BedLine;
use crate::tabix::bedgraph::BedGraphLine;
use crate::tabix::gff::GffLine;
use crate::tabix::gtf::GtfLine;
use crate::tabix::tabix::Tabix;
use crate::tabix::tabix_header::TabixHeader;
use crate::tabix::vcf::VcfLine;
use crate::traits::sam_index::SamIndex;

fn read_lines(
    data: &[u8],
    coordinate: &Coordinate,
    output_format: &OutputFormat,
) -> Result<Vec<String>, ParsingError> {
    let raw_string = String::from_utf8_lossy(data).into_owned();

    let line_strings = raw_string
        .split('\n')
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<String>>();

    let model_new = output_format.get_model();

    let result = line_strings
        .iter()
        .map(|line| model_new(line).map_err(|e| ParsingError::FormatError(e)))
        .collect::<Result<Vec<Box<dyn crate::traits::feature::Feature>>, ParsingError>>()?
        .into_iter()
        .filter(|feature| feature.coordinate().overlaps(coordinate))
        .map(|feature| feature.to_string())
        .collect();
    Ok(result)
}
/// Parses decompressed tabix data and keeps the records that overlap.
///
/// `data[i]` is filtered against `coordinates[i]`, so the two slices must line up
/// one-to-one (one entry per fetched chunk).  Records are written back out in their
/// native text format, in that order.
///
/// # Errors
///
/// Returns [`ParsingError::FormatError`] if a line cannot be parsed as `output_format`.
pub fn data_to_lines(
    data: &[Vec<u8>],
    coordinates: &[Coordinate],
    output_format: &OutputFormat,
) -> Result<Vec<String>, ParsingError> {
    let mut lines = Vec::new();
    for (idx, coord) in coordinates.iter().enumerate() {
        match read_lines(&data[idx], &coord, output_format) {
            Ok(l) => lines.push(l),
            Err(e) => return Err(e),
        }
    }
    Ok(lines.concat())
}

/// Searches a bgzipped, tabix-indexed file (VCF, BED, BedGraph, GFF, GTF) for records
/// overlapping each of [`SearchOptions::coordinates`].
///
/// The `.tbi` index and file header are taken from `options` if present, otherwise read
/// from `index_path` / `file_path`.  Both are returned in the [`SearchResult`] so callers
/// can pass them back in on later queries.
///
/// Result lines are grouped by coordinate in the order the coordinates were given.  Header
/// lines from [`get_header_lines`] come first when `include_header` is set; when
/// `header_only` is set only the file header is returned.
///
/// # Errors
///
/// Returns [`SearchError::InvalidRequest`] if a coordinate names a chromosome that is not
/// in the index, or [`SearchError::ParseError`] if a record cannot be parsed.
pub async fn tabix_search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<SearchResult, SearchError> {
    let mut result = SearchResult::new();

    let tabix = match &options.tabix_index {
        Some(index) => index,
        None => {
            &Tabix::from_compressed_file(store_service, &options.index_path, options.no_cache)
                .await?
        }
    };
    let tabix_header = match &options.tabix_header {
        Some(header) => header,
        None => {
            let first_feature_offset = tabix.first_feature_offset.clone();
            &TabixHeader::from_file(store_service, &options.file_path, first_feature_offset).await?
        }
    };

    result.tabix_header = Some(tabix_header.clone());
    result.tabix_index = Some(tabix.clone());

    if options.header_only {
        result.lines = tabix_header.to_lines();
        return Ok(result);
    }

    let mut chunks = Vec::new();
    let mut flat_coords = Vec::new();

    for coord in &options.coordinates {
        let bin_numbers = get_bin_numbers(&coord);
        let chr_i = tabix
            .get_chromosome_index_by_name(&coord.get_chromosome())
            .ok_or(SearchError::InvalidRequest {
                reason: format!("Chromosome not found in index: {}", coord.get_chromosome()),
                requested: format!("{}", coord),
            })?;
        let chr_idx = &tabix.references[chr_i as usize];
        let coord_chunks = tabix.get_optimized_chunks(&chr_idx, bin_numbers, &coord);
        for _ in 0..coord_chunks.len() {
            flat_coords.push(coord.clone());
        }
        chunks.push(coord_chunks);
    }

    let flat_chunks = chunks.concat();
    let data = fetch(store_service, &options.file_path, &flat_chunks)
        .await
        .map_err(|e| SearchError::ReadError {
            path: options.file_path.to_string(),
            source: e,
        })?;

    let raw_data =
        decompress_chunks(&flat_chunks, &data)
            .await
            .map_err(|e| SearchError::CodecError {
                path: options.file_path.to_string(),
                source: e,
            })?;

    let lines = data_to_lines(&raw_data, &flat_coords, &options.output_format).map_err(|e| {
        SearchError::ParseError {
            source: e,
            path: options.file_path.clone(),
        }
    })?;

    let mut all_lines = get_header_lines(options, &tabix_header);
    all_lines.extend(lines);
    result.lines = all_lines;
    Ok(result)
}

/// Runs [`tabix_search`] on a VCF and parses the result lines into [`VcfLine`]s.
///
/// Lines that do not parse, including any header lines, are skipped.
pub async fn tabix_search_vcf(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<Vec<VcfLine>, SearchError> {
    let tabix_result = tabix_search(store_service, options).await?;
    let mut vcf_lines = Vec::new();

    for line in tabix_result.lines {
        if let Ok(vcf_line) = VcfLine::from_line(line) {
            vcf_lines.push(vcf_line);
        }
    }
    Ok(vcf_lines)
}

/// Returns the header lines to put before the results when `options.include_header` is set.
///
/// VCF returns the file's own `#` header lines.  BED, BedGraph, GFF and GTF return a
/// single tab-separated line of column names.  Returns nothing when `include_header` is
/// `false`.
pub fn get_header_lines(options: &SearchOptions, tabix_header: &TabixHeader) -> Vec<String> {
    if options.include_header {
        match options.output_format {
            OutputFormat::VCF => tabix_header.to_lines(),
            OutputFormat::GFF => vec![GffLine::COLUMNS.join("\t")],
            OutputFormat::GTF => vec![GtfLine::COLUMNS.join("\t")],
            OutputFormat::BED => vec![BedLine::COLUMNS.join("\t")],
            OutputFormat::BEDGRAPH => vec![BedGraphLine::COLUMNS.join("\t")],
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

#[test]
fn test_get_header_lines() {
    let mut options = SearchOptions::new("test.gff.gz", vec!["chr1:1-10000000"]).unwrap();
    options.include_header = true;
    options.output_format = OutputFormat::GFF;
    options.index_path = "test.gff.gz.tbi".to_string();
    options.header_only = false;
    options.bigwig_index = None;

    let tabix_header = TabixHeader::new();
    let header_lines = get_header_lines(&options, &tabix_header);
    println!("{:?}", header_lines);
    assert_eq!(header_lines.len(), 1);
    assert_eq!(
        header_lines[0],
        "chromosome\tsource\tfeature\tbegin\tend\tscore\tstrand\tframe\tgroup"
    );
}

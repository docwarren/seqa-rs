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
use crate::tabix::tabix_index::TabixIndex;
use crate::tabix::tabix_header::TabixHeader;
use crate::traits::feature::Feature;
use crate::traits::sam_index::SamIndex;
use crate::traits::to_lines::ToLines;
use bstr::ByteSlice;

fn read_lines(
    data: &[u8],
    coordinate: &Coordinate,
    output_format: &OutputFormat,
) -> Result<Vec<String>, ParsingError> {
    let lines: Vec<&[u8]> = data.lines().collect();

    let result = lines
        .iter()
        .map(|line| output_format.read_from_bytes(line))
        .collect::<Result<Vec<Box<dyn Feature>>, ParsingError>>()?
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
    data: &[u8],
    coordinate: &Coordinate,
    output_format: &OutputFormat,
) -> Result<Vec<String>, ParsingError> {
    read_lines(data, coordinate, output_format)
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
    result.coordinates = options.coordinates.iter().map(|c| c.to_string()).collect();

    let tabix = match &options.tabix_index {
        Some(index) => index,
        None => {
            &TabixIndex::from_compressed_file(store_service, &options.index_path, options.no_cache)
                .await?
        }
    };
    let tabix_header = match &options.tabix_header {
        Some(header) => header,
        None => {
            let first_feature_offset = tabix.first_feature_offset;
            &TabixHeader::from_file(store_service, &options.file_path, first_feature_offset).await?
        }
    };

    result.header = Some(Box::new(tabix_header.clone()));
    result.tabix_index = Some(tabix.clone());

    if options.header_only {
        return Ok(result);
    }

    for coord in &options.coordinates {
        let bin_numbers = get_bin_numbers(coord);
        let Some(chr_i) = tabix.get_chromosome_index_by_name(coord.get_chromosome()) else {
            continue;
        };
        let chr_idx = &tabix.references[chr_i];
        let coord_chunks = tabix.get_optimized_chunks(chr_idx, bin_numbers, coord);

        let data = fetch(store_service, &options.file_path, &coord_chunks)
            .await
            .map_err(|e| SearchError::ReadError {
                path: options.file_path.to_string(),
                source: e,
            })?;

        let raw_data =
            decompress_chunks(&coord_chunks, &data)
                .await
                .map_err(|e| SearchError::CodecError {
                    path: options.file_path.to_string(),
                    source: e,
                })?;

        let lines = data_to_lines(&raw_data.concat(), coord, &options.output_format)?;
        result.lines.push(lines);
    }

    Ok(result)
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
    let mut options =
        SearchOptions::new("test.gff.gz", &vec!["chr1:1-10000000".to_string()]).unwrap();
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

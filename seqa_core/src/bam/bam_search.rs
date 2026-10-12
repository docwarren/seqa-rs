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
use crate::api::search::{fetch, join_fetched_data};
use crate::api::search_error::SearchError;
use crate::api::search_options::{CigarFormat, SearchOptions};
use crate::api::search_result::SearchResult;
use crate::bam::bai::BaiIndex;
use crate::bam::header::header::BamHeader;
use crate::bam::read::Read;
use crate::coordinate::{check_coords_for_length, Coordinate};
use crate::indexes::bin_util::get_bin_numbers;
use crate::stores::StoreService;
use crate::traits::from_bytes::FromBytes;
use crate::traits::sam_index::SamIndex;

const MAX_BAM_SEARCH_SIZE: usize = 200_000;

/// Converts raw data bytes into a vector of strings, processing each line according to the search options.
/// # Arguments:
/// * `data` - A vector of bytes representing the raw data to be processed.
/// * `options` - A `SearchOptions` struct containing the search parameters such as output format,
///  whether to include headers, and the range of positions to consider.
/// # Returns:
/// * A vector of strings containing the processed lines, which may include headers based on the options.
pub fn data_to_lines(
    data: &[u8],
    coordinate: &Coordinate,
    cigar_format: &CigarFormat,
) -> Result<(bool, Vec<String>), SearchError> {
    let mut lines = Vec::new();
    let mut i = 0;
    let mut end = false;
    let use_merged_cigar = *cigar_format == CigarFormat::Merged;
    while let Ok(read_line_result) = Read::from_bytes(data, i) {
        let read_line = read_line_result.0;
        let j = read_line_result.1;
        if read_line.pos > coordinate.get_end() as i32 {
            end = true;
            break; // Stop processing if the position exceeds the end of the search range
        } else if read_line.get_end() < (coordinate.get_begin()) {
            i = j; // Skip this read if it is before the start of the search range
            continue;
        } else {
            lines.push(read_line.to_sam_string(use_merged_cigar));
        }
        i = j;
    }

    Ok((end, lines))
}

/// Searches for data in a bam based on the provided search options.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the search parameters such as file paths, chromosome,
///  start and end positions, output format, and whether to include headers or only headers.
/// # Returns:
/// * A Result containing a vector of strings with the search results, or an error message if the search fails.
pub async fn bam_search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<SearchResult, SearchError> {
    if let Err(e) = check_coords_for_length(&options.coordinates, MAX_BAM_SEARCH_SIZE) {
        return Err(e);
    }

    let mut result = SearchResult::new();

    let bai = match &options.bam_index {
        Some(index) => index,
        None => &BaiIndex::from_file(store_service, &options.index_path, options.no_cache).await?
    };

    let first_feature_offset = bai.get_first_feature_offset().await;
    let bam_header = match &options.bam_header {
        Some(header) => header,
        None => &BamHeader::from_file(store_service, &options.file_path, first_feature_offset).await?
    };

    result.bam_index = Some(bai.clone());
    result.bam_header = Some(bam_header.clone());

    if options.header_only {
        result.lines = bam_header.to_lines();
        return Ok(result);
    }

    let mut start_lines = if options.include_header {
        bam_header.to_lines()
    } else {
        Vec::new()
    };

    for coord in &options.coordinates {

        let bin_numbers = get_bin_numbers(&coord);

        let chr_i = bam_header
            .get_chromosome_index_by_name(&coord.get_chromosome())
            .ok_or_else(|| SearchError::InvalidRequest {
                requested: coord.get_chromosome().to_string(),
                reason: "Chromosome not found in index".to_string()
            })?;

        let chr_idx = &bai.references[chr_i];
        let chunks = bai.get_optimized_chunks(&chr_idx, bin_numbers, &coord);

        let data = fetch(store_service, &options.file_path, &chunks)
            .await
            .map_err(|e| SearchError::ReadError {
                path: options.file_path.to_string(),
                source: e
            })?;

        let raw_data = join_fetched_data(&chunks, &data)
            .await
            .map_err(|e| SearchError::CodecError {
                path: options.file_path.to_string(),
                source: e
            })?;

        let lines = data_to_lines(&raw_data.concat(), &coord, &options.cigar_format)?;
        start_lines.extend(lines.1)
    }
    result.lines = start_lines;
    Ok(result)
}

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
use crate::api::search::{decompress_chunks, fetch};
use crate::api::search_error::SearchError;
use crate::api::search_options::{CigarFormat, SearchOptions};
use crate::api::search_result::SearchResult;
use crate::bam::bai::BaiIndex;
use crate::bam::bam_header::header::BamHeader;
use crate::bam::read::Read;
use crate::coordinate::{Coordinate, check_coord_for_length};
use crate::indexes::bin_util::get_bin_numbers;
use crate::stores::StoreService;
use crate::traits::feature::Feature;
use crate::traits::from_bytes::FromBytes;
use crate::traits::sam_index::SamIndex;

const MAX_BAM_SEARCH_SIZE: usize = 200_001;

fn read_lines(
    data: &[u8],
    coordinate: &Coordinate,
    cigar_format: &CigarFormat,
    bam_header: &BamHeader,
) -> Result<Vec<String>, SearchError> {
    let mut lines = Vec::new();
    let mut i = 0;
    let use_merged_cigar = *cigar_format == CigarFormat::Merged;
    while let Ok(read_line_result) = Read::from_bytes(&data[i..]) {
        let read_line = read_line_result.0;
        let j = read_line_result.1;
        if read_line.pos >= coordinate.get_end() as i32 {
            break; // Stop processing if the position exceeds the end of the search range
        } else if !read_line
            .coordinate()
            .overlaps_ignore_chromosome(coordinate)
        {
            i += j; // Skip this read if it is before the start of the search range
            continue;
        } else {
            lines.push(read_line.to_sam_string(use_merged_cigar, bam_header));
        }
        i += j;
    }

    Ok(lines)
}

/// Converts decompressed BAM data into SAM lines.
///
/// `data[i]` is searched for reads overlapping `coordinate[i]`, so the two slices must
/// line up one-to-one (one entry per fetched chunk).  Lines are returned in that order.
pub fn data_to_lines(
    data: &[u8],
    coordinate: &Coordinate,
    cigar_format: &CigarFormat,
    bam_header: &BamHeader,
) -> Result<Vec<String>, SearchError> {
    read_lines(data, coordinate, cigar_format, bam_header)
}

/// Searches a BAM file for reads overlapping each of [`SearchOptions::coordinates`].
///
/// The `.bai` index and BAM header are taken from `options` if present, otherwise read
/// from `index_path` / `file_path`.  Both are returned in the [`SearchResult`] so callers
/// can pass them back in on later queries.
///
/// `lines` has one entry per coordinate, in the order the coordinates were given, holding
/// the SAM records that overlap it.  The entry is empty when the coordinate covers more
/// than 200,001 bases or names a chromosome that is not in the BAM header.  Overlapping
/// coordinates can return the same read more than once.  When `header_only` is set every
/// entry is empty.  The SAM header is always returned in [`SearchResult::header`].
pub async fn bam_search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<SearchResult, SearchError> {

    let mut result = SearchResult::new();
    result.coordinates = options.coordinates.iter().map(|c| c.to_string()).collect();

    let bai = match &options.bam_index {
        Some(index) => index,
        None => &BaiIndex::from_file(store_service, &options.index_path, options.no_cache).await?,
    };

    let first_feature_offset = bai.get_first_feature_offset().await;
    let bam_header = match &options.bam_header {
        Some(header) => header,
        None => {
            &BamHeader::from_file(store_service, &options.file_path, first_feature_offset).await?
        }
    };

    result.bam_index = Some(bai.clone());
    result.header = Some(Box::new(bam_header.clone()));

    if options.header_only {
        result.lines = vec![Vec::new(); options.coordinates.len()];
        return Ok(result);
    }

    for coord in &options.coordinates {
        if !check_coord_for_length(coord, MAX_BAM_SEARCH_SIZE) {
            result.lines.push(Vec::new());
            continue;
        }
        let bin_numbers = get_bin_numbers(coord);
        let Some(chr_i) = bam_header.get_chromosome_index_by_name(coord.get_chromosome()) else {
            result.lines.push(Vec::new());
            continue;
        };
        let chr_idx = &bai.references[chr_i];
        let chunks = bai.get_optimized_chunks(chr_idx, bin_numbers, coord);

        let compressed = fetch(store_service, &options.file_path, &chunks)
            .await
            .map_err(|e| SearchError::ReadError {
                path: options.file_path.to_string(),
                source: e,
            })?;

        let raw_data = decompress_chunks(&chunks, &compressed)
            .await
            .map_err(|e| SearchError::CodecError {
                path: options.file_path.to_string(),
                source: e,
            })?;

        let lines = data_to_lines(&raw_data.concat(), coord, &options.cigar_format, bam_header)?;
        result.lines.push(lines);
    }

    Ok(result)
}

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
use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;
use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::coordinate::check_coords_for_length;
use crate::fasta::fai::FaiIndex;
use crate::stores::StoreService;
use std::ops::Range;

const MAX_FASTA_SEARCH_SIZE: usize = 250_000_000;

/// Reads the sequence for each of [`SearchOptions::coordinates`] from an indexed FASTA file.
///
/// The `.fai` index is taken from `options` if present, otherwise read from `index_path`,
/// and is returned in the [`SearchResult`].  Coordinates are 1-based closed, so
/// `chr1:1-100` returns 100 bases.
///
/// Result lines are the sequence lines as stored in the file, with no `>` header line,
/// one coordinate after another in the order given.  `include_header` and `header_only`
/// are ignored.
///
/// # Errors
///
/// Returns [`SearchError::InvalidRequest`] if a coordinate covers more than 250,000,000
/// bases, names a contig that is not in the index, or falls outside the contig.
pub async fn fasta_search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<SearchResult, SearchError> {
    let mut result = SearchResult::new();
    result.coordinates = options.coordinates.iter().map(|c| c.to_string()).collect();

    check_coords_for_length(&options.coordinates, MAX_FASTA_SEARCH_SIZE)?;

    let index = match &options.fasta_index {
        Some(index) => index,
        None => &FaiIndex::from_file(store_service, &options.index_path).await?,
    };
    result.fasta_index = Some(index.clone());

    let byte_ranges: Vec<Range<u64>> = index.get_offsets(&options.coordinates)?;

    let bytes = store_service
        .get_ranges(&options.file_path, &byte_ranges)
        .await
        .map_err(|e| SearchError::ReadError {
            path: options.file_path.to_string(),
            source: e,
        })?;

    result.lines = bytes
        .iter()
        .map(|chunk| {
            let data = String::from_utf8(chunk.clone()).map_err(ParsingError::StringParseError)?;

            Ok::<Vec<String>, SearchError>(
                data.split('\n')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            )
        })
        .collect::<Result<Vec<Vec<String>>, SearchError>>()?;

    Ok(result)
}

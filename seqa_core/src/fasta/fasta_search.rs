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
use std::ops::Range;
use crate::api::parsing_error::ParsingError;
use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::stores::StoreService;
use crate::api::search_error::SearchError;
use crate::coordinate::check_coords_for_length;
use crate::fasta::fai::FaiIndex;

const MAX_FASTA_SEARCH_SIZE: usize = 250_000_000;

/// Searches for data in a FASTA file based on the provided search options.
/// Returns a vector of strings containing the results.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the search parameters such as file
///   paths, chromosome, start and end positions, output format, and whether to include headers or only headers.
/// # Returns:
/// * A Result containing a vector of strings with the search results, or an error message if the search fails.
pub async fn fasta_search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<SearchResult, SearchError> {
    let mut result = SearchResult::new();

    if let Err(e) = check_coords_for_length(&options.coordinates, MAX_FASTA_SEARCH_SIZE) {
        return Err(e);
    }

    let index = match &options.fasta_index {
        Some(index) => index,
        None => &FaiIndex::from_file(store_service, &options.index_path).await?
    };
    result.fasta_index = Some(index.clone());

    let byte_ranges: Vec<Range<u64>> = index.get_offsets(&options.coordinates)?;

    let bytes = store_service.get_ranges(&options.file_path, &byte_ranges)
        .await
        .map_err(|e| SearchError::ReadError {
            path: options.file_path.to_string(),
            source: e
        })?;

    let lines =  bytes.iter().map(|chunk| {
        let data = String::from_utf8(chunk.clone()).map_err(|e| SearchError::ParseError {
            path: options.file_path.to_string(),
            source: ParsingError::StringParseError(e)
        })?;

        Ok::<Vec<String>, SearchError>(data.split('\n').map(|s| s.to_string()).collect::<Vec<String>>())
    }).collect::<Result<Vec<Vec<String>>, SearchError>>()?;
    result.lines = lines.into_iter().flatten().collect();

    Ok(result)
}

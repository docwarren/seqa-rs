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
use crate::api::search_error::SearchError;
use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::bam::bam_search::bam_search;
use crate::bigwig::bigbed_search::bigbed_search;
use crate::bigwig::bigwig_search::bigwig_search;
use crate::codecs::bgzip;
use crate::codecs::codec_error::CodecError;
use crate::fasta::fasta_search::fasta_search;
use crate::indexes::chunk::Chunk;
use crate::stores::StoreService;
use crate::stores::error::StoreError;
use crate::tabix::tabix_search::tabix_search;

/// Fetches the compressed byte range for every chunk in a single
/// [`StoreService::get_ranges`] call.
///
/// Each range runs from the chunk's first block to one maximum block size past its
/// last block, so the last block is always fetched whole.  Returns one buffer per
/// chunk, in the same order as `chunks`.
pub(crate) async fn fetch(
    store_service: &StoreService,
    file_path: &str,
    chunks: &[Chunk],
) -> Result<Vec<Vec<u8>>, StoreError> {
    let ranges = chunks.iter().map(|c| c.to_range()).collect::<Vec<_>>();
    let data = store_service.get_ranges(file_path, &ranges).await?;
    Ok(data)
}

/// Decompresses each fetched chunk and drops the bytes before the chunk's first record.
///
/// `fetched[i]` must be the data returned by [`fetch`] for `chunks[i]`.  Returns one
/// buffer per chunk, starting at `begin_vp`.  The tail is not trimmed, so the buffer may
/// end part way through a record.
pub(crate) async fn decompress_chunks(
    chunks: &[Chunk],
    fetched: &[Vec<u8>],
) -> Result<Vec<Vec<u8>>, CodecError> {
    let mut raw_data = Vec::new();

    for i in 0..chunks.len() {
        let chunk = chunks[i];
        let compressed_bytes = &fetched[i];
        let block_sizes = bgzip::from_bytes(compressed_bytes)?;
        let decompressed_bytes = bgzip::decompress(&block_sizes, compressed_bytes)?;
        let decompressed_start_byte = chunk.begin_vp.decompressed_offset as usize;
        let decompressed_slice = &decompressed_bytes[decompressed_start_byte..];
        raw_data.push(decompressed_slice.to_vec());
    }
    Ok(raw_data)
}

/// Searches for features in a file based on the provided search options.
///
/// Dispatches to the format-specific search function indicated by
/// [`SearchOptions::output_format`], searching every coordinate in
/// [`SearchOptions::coordinates`].  Backend clients are reused from `store`'s cache.
///
/// # Errors
///
/// Returns [`SearchError::UnsupportedFileFormat`] for [`OutputFormat::STRING`], otherwise
/// whatever the format-specific search returns.
pub async fn search_features(
    store: &StoreService,
    options: &SearchOptions,
) -> Result<SearchResult, SearchError> {
    match options.output_format {
        OutputFormat::BAM => bam_search(store, options).await,
        OutputFormat::BIGWIG => bigwig_search(store, options).await,
        OutputFormat::BIGBED => bigbed_search(store, options).await,
        OutputFormat::VCF
        | OutputFormat::BED
        | OutputFormat::BEDGRAPH
        | OutputFormat::GFF
        | OutputFormat::GTF => tabix_search(store, options).await,
        OutputFormat::FASTA => fasta_search(store, options).await,
        _ => Err(SearchError::UnsupportedFileFormat(
            options.file_path.to_string(),
        )),
    }
}

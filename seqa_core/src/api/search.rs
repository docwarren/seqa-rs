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

use crate::api::bam_search::{bam_search, BamError};
use crate::api::bigbed_search::{bigbed_search, BigbedError};
use crate::api::bigwig_search::{bigwig_search, BigwigError};
use crate::api::fasta_search::{fasta_search, FastaSearchError};
use crate::api::output_format::OutputFormat;
use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::api::tabix_search::{tabix_search, TabixSearchError};
use crate::codecs::bgzip;
use crate::indexes::chunk::Chunk;
use crate::stores::StoreService;
use crate::utils::UtilError;
use futures::{future::join_all, TryStreamExt};
use log::error;
use object_store::ObjectStore;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Failed to process data: {0}")]
    DataProcessingError(String),

    #[error("Store Error: {0}")]
    StoreError(#[from] crate::stores::error::StoreError),

    #[error("Object Store Error: {0}")]
    ObjectStoreError(#[from] object_store::Error),

    #[error("BgZip Error: {0}")]
    BgZipError(#[from] bgzip::BgZipError),
}

/// Unified error returned by [`StoreService::search_features`].
///
/// Wraps the format-specific error from the underlying search function.
#[derive(Debug, Error)]
pub enum SearchFeaturesError {
    #[error("Search Error: {0}")]
    String(String),

    #[error("BAM error occurred")]
    Bam(#[from] BamError),

    #[error("Fasta error occurred")]
    Fasta(#[from] FastaSearchError),

    #[error("Tabix error occurred")]
    Tabix(#[from] TabixSearchError),

    #[error("BigWig error occurred")]
    BigWig(#[from] BigwigError),

    #[error("BigBed error occurred")]
    BigBed(#[from] BigbedError),

    #[error("Utility error occurred")]
    Util(#[from] UtilError),
}

pub async fn chunk_to_stream(
    chunk: &Chunk,
    store: &dyn ObjectStore,
    path: &object_store::path::Path,

) -> Result<impl futures::Stream<Item = Result<Vec<u8>, object_store::Error>>, object_store::Error> {

    let range = chunk.to_range();

    let get_opts = object_store::GetOptions {
        range: Some(object_store::GetRange::Bounded(range.clone())),
        ..Default::default()
    };

    let stream = store
        .get_opts(path, get_opts)
        .await?
        .into_stream()
        .map_ok(|bytes| bytes.to_vec());

    Ok(stream.map_ok(|get_result| get_result.to_vec()))
}

/// Fetches chunks concurrently from the store service.
///
/// Returns a vector of `(Chunk, bytes)` pairs preserving input order.
pub async fn init_fetch_handles(
    store_service: &StoreService,
    options: &SearchOptions,
    chunks: &[Chunk],
) -> Result<Vec<(Chunk, Vec<u8>)>, SearchError> {
    let futures = chunks.iter().map(|chunk| {
        let range = chunk.to_range();
        let file_path = options.file_path.clone();
        let chunk_clone = chunk.clone();
        async move {
            match store_service.get_range(&file_path, range).await {
                Ok(data) => (chunk_clone, data),
                Err(e) => {
                    error!("Error fetching range for chunk {:?}: {}", chunk_clone, e);
                    (chunk_clone, vec![])
                }
            }
        }
    });

    Ok(join_all(futures).await)
}

/// Decompresses each fetched chunk and returns the post-offset data.
pub async fn join_fetch_handles(
    fetched: Vec<(Chunk, Vec<u8>)>,
) -> Result<Vec<Vec<u8>>, SearchError> {
    let mut raw_data = Vec::new();

    for (chunk, compressed_bytes) in fetched {
        let block_sizes = bgzip::from_bytes(&compressed_bytes)?;
        let decompressed_bytes = bgzip::decompress(&block_sizes, &compressed_bytes)?;
        let decompressed_start_byte = chunk.begin_vp.decompressed_offset as usize;
        let decompressed_slice = &decompressed_bytes[decompressed_start_byte..];
        raw_data.push(decompressed_slice.to_vec());
    }
    Ok(raw_data)
}

impl StoreService {
    /// Searches for features in a file based on the provided search options.
    ///
    /// Dispatches to the format-specific search function indicated by
    /// [`SearchOptions::output_format`] and reuses this `StoreService`'s cached
    /// object_store clients for every backend access.
    pub async fn search_features(
        &self,
        options: &SearchOptions,
    ) -> Result<SearchResult, SearchFeaturesError> {
        let result = match options.output_format {
            OutputFormat::BAM => bam_search(self, options).await.map_err(SearchFeaturesError::from),
            OutputFormat::BIGWIG => bigwig_search(self, options).await.map_err(SearchFeaturesError::from),
            OutputFormat::BIGBED => bigbed_search(self, options).await.map_err(SearchFeaturesError::from),
            OutputFormat::VCF
            | OutputFormat::BED
            | OutputFormat::BEDGRAPH
            | OutputFormat::GFF
            | OutputFormat::GTF => tabix_search(self, options).await.map_err(SearchFeaturesError::from),
            OutputFormat::FASTA => fasta_search(self, options).await.map_err(SearchFeaturesError::from),
            _ => Err(SearchFeaturesError::String(
                "Output format is not supported for file search".into(),
            )),
        };

        result.map_err(|e| SearchFeaturesError::String(format!("Error searching file: {}", e)))
    }
}
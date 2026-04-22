use std::ops::Range;
use thiserror::Error;

use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::indexes::fai::{FaiIndex, FaiIndexError};
use crate::stores::error::StoreError;
use crate::stores::StoreService;

#[derive(Debug, Error)]
pub enum FastaSearchError {
    #[error("FAI index error: {0}")]
    FaiIndexError(#[from] FaiIndexError),

    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Store Error: {0}")]
    StoreError(#[from] StoreError),

    #[error("Failed to read FASTA file: {0}")]
    FailedToReadFastaFile(String),
}

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
) -> Result<SearchResult, FastaSearchError> {
    let mut result = SearchResult::new();

    if options.end - options.begin > 100_000 {
        return Err(FastaSearchError::FailedToReadFastaFile(
            "Requested range is too large; please limit to 100,000 bases.".into(),
        ));
    }
    let index = match &options.fasta_index {
        Some(index) => index,
        None => &FaiIndex::from_file(store_service, &options.index_path).await?
    };
    result.fasta_index = Some(index.clone());

    let byte_range: Range<u64> = index.get_offsets(&options)?;

    let bytes = store_service.get_range(&options.file_path, byte_range).await?;
    let line_string = String::from_utf8(bytes)?;
    result.lines = line_string.lines()
        .map(|line| line.to_string())
        .collect::<Vec<String>>();
    Ok(result)
}

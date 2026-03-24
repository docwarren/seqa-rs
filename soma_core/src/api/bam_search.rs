use thiserror::Error;

use super::search_options::{CigarFormat, SearchOptions};
use crate::api::search::stream_data_to_strings;
use crate::api::search_result::SearchResult;
use crate::indexes::bai::{BaiError, BaiIndex};
use crate::indexes::bin_util::get_bin_numbers;
use crate::indexes::traits::sam_index::SamIndex;
use crate::models::bam::read::Read;
use crate::models::bam_header::header::{BamHeader, BamHeaderError};
use crate::traits::feature::Feature;

#[derive(Debug, Error)]
pub enum BamError {

    #[error("Data processing error: {0}")]
    DataProcessingError(String),

    #[error("Chromosome not found: {0}")]
    ChromosomeNotFound(String),

    #[error("Failed to read BAM header: {0}")]
    HeaderError(#[from] BamHeaderError),

    #[error("Failed to read Bai index: {0}")]
    BaiError(#[from] BaiError),
}

/// Converts raw data bytes into a vector of strings, processing each line according to the search options.
/// # Arguments:
/// * `data` - A vector of bytes representing the raw data to be processed.
/// * `options` - A `SearchOptions` struct containing the search parameters such as output format,
///  whether to include headers, and the range of positions to consider.
/// # Returns:
/// * A vector of strings containing the processed lines, which may include headers based on the options.
pub fn data_to_lines(
    data: &Vec<u8>,
    options: &SearchOptions,
    bam_header: &BamHeader,
) -> Result<(bool, Vec<String>), BamError> {
    let mut lines = Vec::new();
    let mut i = 0;
    let mut end = false;
    let use_merged_cigar = options.cigar_format == CigarFormat::Merged;

    loop {
        match Read::from_bytes(data, i, bam_header) {
            Ok((read_line, j)) => {
                if read_line.pos > options.end as i32 {
                    end = true;
                    break; // Stop processing if the position exceeds the end of the search range
                } else if !read_line.overlaps(options) {
                    i = j; // Skip this read if it is before the start of the search range
                    continue;
                } else {
                    lines.push(read_line.to_sam_string(use_merged_cigar));
                }
                i = j;
            }
            _ => {
                break;
            }
        }
    }

    Ok((end, lines))
}

/// Searches for data in a bam based on the provided search options.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the search parameters such as file paths, chromosome,
///  start and end positions, output format, and whether to include headers or only headers.
/// # Returns:
/// * A Result containing a vector of strings with the search results, or an error message if the search fails.
pub async fn bam_search(options: &SearchOptions) -> Result<SearchResult, BamError> {
    let mut result = SearchResult::new();

    if options.end - options.begin > 200_000 {
        return Err(BamError::DataProcessingError(
            "Requested range is too large; please limit to 100,000 bases.".into(),
        ));
    }

    let bai = match &options.bam_index {
        Some(index) => index,
        None => &BaiIndex::from_file_with_data_path(&options.index_path, &options.file_path).await?
    };


    let first_feature_offset = bai.get_first_feature_offset().await;
    let bam_header = match &options.bam_header {
        Some(header) => header,
        None => &BamHeader::from_file(&options.file_path, first_feature_offset).await?
    };

    result.bam_index = Some(bai.clone());
    result.bam_header = Some(bam_header.clone());

    if options.header_only {
        result.lines = bam_header.to_lines();
        return Ok(result);
    }

    let start_lines = if options.include_header {
        bam_header.to_lines()
    } else {
        Vec::new()
    };

    let bin_numbers = get_bin_numbers(options.begin, options.end);

    let chr_i = bam_header
        .get_chromosome_index_by_name(&options.chromosome)
        .ok_or_else(|| BamError::ChromosomeNotFound(options.chromosome.clone()))?;

    let chr_idx = &bai.references[chr_i as usize];
    let chunks = bai.get_optimized_chunks(&chr_idx, bin_numbers, &options);

    let lines = stream_data_to_strings(&options, start_lines, &chunks, |data| {
        match data_to_lines(data, options, &bam_header) {
            Ok((end, lines)) => Ok((end, lines)),
            Err(e) => Err(e.to_string())
        }
    })
    .await
    .map_err(|e| BamError::DataProcessingError(format!("Failed to process data: {}", e)));

    match lines {
        Ok(lines) => {
            result.lines = lines;
            Ok(result)
        }
        Err(e) => Err(e)
    }
}

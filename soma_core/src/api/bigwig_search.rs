use std::ops::Range;
use thiserror::Error;
use crate::codecs::decompress_auto;
use crate::indexes::bigwig::chr_tree::BigwigChrTree;
use crate::indexes::bigwig::r_tree::overlaps::Overlaps;
use crate::models::bigwig::bigwig_data::BigwigData;
use crate::models::bigwig::zoom_data::ZoomData;
use crate::indexes::bigwig::{BigwigIndex, BigwigIndexError};
use crate::indexes::bigwig::r_tree::r_tree_leaf::RTreeLeaf;
use crate::indexes::bigwig::r_tree::{RTree, RTreeError};
use crate::indexes::bigwig::zoom_header::ZoomHeader;
use super::search_options::SearchOptions;
use super::search_result::SearchResult;

#[derive(Debug, Error)]
pub enum BigwigError {
    #[error("Data processing error: {0}")]
    DataProcessingError(String),

    #[error("Bigwig index error: {0}")]
    BigwigIndexError(#[from] BigwigIndexError),

    #[error("RTree Error: {0}")]
    RTreeError(#[from] RTreeError),
}

pub fn get_zoom_strings(bytes: Vec<u8>, chr_tree: &BigwigChrTree, options: &SearchOptions) -> Vec<String> {
    let mut str_array = Vec::new();
    let mut offset = 0;
    let chrom_id = chr_tree.get_chromosome_id(&options.chromosome).unwrap_or(0);

    while offset + ZoomData::SIZE <= bytes.len() {
        if let Ok(zoom_data) = ZoomData::from_bytes(&bytes[offset..offset + ZoomData::SIZE], chr_tree) {
            if zoom_data.overlaps(chrom_id, chrom_id, options.begin, options.end) {
                str_array.push(format!("{}", zoom_data));
            }
            offset += ZoomData::SIZE;
        } else {
            break; // Exit if we can't parse a complete ZoomData
        }
    }

    str_array
}

pub fn get_data_strings(bytes: &[u8], chr_tree: &BigwigChrTree, options: &SearchOptions) -> Vec<String> {
    let mut str_array = Vec::new();
    let mut offset = 0;
    let chrom_id = chr_tree.get_chromosome_id(&options.chromosome).unwrap_or(0);

    while offset < bytes.len() + BigwigData::SIZE {
        if let Ok(bigwig_data) = BigwigData::from_bytes(&bytes[offset..], chr_tree) {
            offset += bigwig_data.size;
            for data_point in &bigwig_data.data {
                if data_point.overlaps(chrom_id, chrom_id, options.begin, options.end) {
                    str_array.push(format!("{}", data_point));
                }
            }

        } else {
            break; // Exit if we can't parse a complete RTreeLeaf
        }
    }

    str_array
}
/// Converts raw data bytes into a vector of strings, processing each line according to the search options.
/// # Arguments:
/// * `data` - A vector of bytes representing the raw data to be processed.
/// * `options` - A `SearchOptions` struct containing the search parameters such as output format,
///  whether to include headers, and the range of positions to consider.
/// # Returns:
/// * A vector of strings containing the processed lines, which may include headers.
pub fn data_to_lines(bytes: Vec<Vec<u8>>, is_zoom: bool, chr_tree: &BigwigChrTree, options: &SearchOptions) -> Result<Vec<String>, BigwigError> {

    let mut str_array: Vec<String> = if options.include_header {
        if is_zoom {
            vec!(ZoomData::COLUMNS.iter().map(|s| s.to_string()).collect::<Vec<String>>().join("\t"))
        } else {
            vec!(BigwigData::COLUMNS.iter().map(|s| s.to_string()).collect::<Vec<String>>().join("\t"))
        }
    } else {
        Vec::new()
    };

    for block in bytes {
        let strings = if is_zoom {
            get_zoom_strings(block, chr_tree, options)
        } else {
            get_data_strings(&block.to_vec(), chr_tree, options)
        };
        str_array.extend(strings);
    }
    Ok(str_array)
}

/// Searches for data in a bigwig file based on the provided search options.
/// Returns a vector of strings containing the results, which may include headers.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the search parameters such as file paths, chromosome,
///  start and end positions, output format, and whether to include headers or only headers.
/// # Returns:
/// * A Result containing a vector of strings with the search results, or an error message if the search fails.
pub async fn bigwig_search(options: &SearchOptions) -> Result<SearchResult, BigwigError> {
    let mut result = SearchResult::new();

    let index = match &options.bigwig_index {
        Some(index) => {
            index
        },
        _ => &BigwigIndex::new(&options.file_path).await?
    };
    result.bigwig_index = Some(index.clone());

    let zoom_header = index.get_zoom_header(options);
    let is_compressed = index.header.is_compressed();

    let chr_id = match index.chromosome_tree.get_chromosome_id(&options.chromosome) {
        Some(id) => id,
        _ => {
            return Err(BigwigError::DataProcessingError(format!("Chromosome not found: {}", options.chromosome)));
        }
    };

    let index_offset = get_index_begin(&index, zoom_header).await?;
    let index_end = get_index_end(&index, zoom_header).await?;

    let r_tree = RTree::from_file(&options.file_path, index_offset..index_end)
        .await?;

    let leaves = r_tree.get_overlapping_leaves(chr_id, options.begin, options.end);

    if leaves.len() == 0 {
        return Ok(result);
    }

    let range = get_range_from_leaves(&leaves);

    let data = index.get_data(&range, &options.file_path).await?;

    let mut decompressed_blocks: Vec<Vec<u8>> = Vec::new();

    for leaf in leaves {
        let begin = (leaf.data_offset - range.start) as usize;
        let end = (begin + leaf.data_size as usize) as usize;
        let compressed = data[begin..end].to_vec();
        if is_compressed {
            match decompress_auto(&compressed) {
                Ok(decompressed) => decompressed_blocks.push(decompressed),
                Err(_e) => {
                    continue;
                }
            }
        } else {
            decompressed_blocks.push(compressed);
        };
    }
    let is_zoom = match zoom_header {
        Some(_) => true,
        _ => false,
    };

    result.lines = match data_to_lines(decompressed_blocks, is_zoom, &index.chromosome_tree, options) {
        Ok(lines) => lines,
        Err(e) => return Err(e)
    };

    Ok(result)
}

async fn get_index_begin(index: &BigwigIndex, zoom_header: Option<&ZoomHeader>) -> Result<u64, BigwigError> {
    match zoom_header {
        Some(zoom_header) => Ok(zoom_header.index_offset as u64),
        None => Ok(index.header.full_index_offset as u64),
    }
}

async fn get_index_end(index: &BigwigIndex, zoom_header: Option<&ZoomHeader>) -> Result<u64, BigwigError> {
    match zoom_header {
        Some(zoom_header) => Ok(index.get_end_for_zoom_header(zoom_header, &index.file_path).await?),
        None => Ok(index.get_full_index_end(&index.file_path).await?),
    }
}

pub fn get_range_from_leaves(leaves: &[&RTreeLeaf]) -> Range<u64> {
    let begin = leaves.iter().map(|l| l.data_offset).min().unwrap_or(u64::MAX);
    let end = leaves
        .iter()
        .map(|l| l.data_offset + l.data_size)
        .max()
        .unwrap_or(u64::MAX);
    begin..end
}

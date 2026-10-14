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
use crate::api::search_error::SearchError;
use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::bigwig::bigwig_data::BigwigData;
use crate::bigwig::index::bigwig_index::BigwigIndex;
use crate::bigwig::index::chr_tree::BigwigChrTree;
use crate::bigwig::index::r_tree::RTree;
use crate::bigwig::index::r_tree::overlaps::Overlaps;
use crate::bigwig::index::r_tree::r_tree_leaf::RTreeLeaf;
use crate::bigwig::index::util::get_zoom_strings;
use crate::bigwig::index::zoom_header::ZoomHeader;
use crate::codecs::codec_error::CodecError;
use crate::codecs::decompress_auto;
use crate::coordinate::Coordinate;
use crate::stores::StoreService;
use std::ops::Range;

/// Parses a decompressed BigWig data block and formats each data point that overlaps `coord`.
pub fn get_data_strings(bytes: &[u8], chr_tree: &BigwigChrTree, coord: &Coordinate) -> Vec<String> {
    let mut str_array = Vec::new();
    let mut offset = 0;
    let chrom_id = chr_tree
        .get_chromosome_id(&coord.get_chromosome())
        .unwrap_or(0);

    while offset < bytes.len() + BigwigData::SIZE {
        if let Ok(bigwig_data) = BigwigData::from_bytes(&bytes[offset..], chr_tree) {
            offset += bigwig_data.size;
            for data_point in &bigwig_data.data {
                if data_point.overlaps(chrom_id, chrom_id, coord.get_begin(), coord.get_end()) {
                    str_array.push(format!("{}", data_point));
                }
            }
        } else {
            break; // Exit if we can't parse a complete RTreeLeaf
        }
    }

    str_array
}
/// Formats the decompressed blocks for one coordinate as text lines.
///
/// Zoom blocks are formatted as zoom summaries, otherwise as data points.  When
/// `include_header` is set, the matching column-name line comes first.
fn data_to_lines(
    bytes: &[Vec<u8>],
    chr_tree: &BigwigChrTree,
    coordinate: &Coordinate,
    is_zoom: bool,
) -> Vec<String> {
    let mut str_array = vec![];

    for block in bytes {
        let strings = if is_zoom {
            get_zoom_strings(&block, chr_tree, coordinate)
        } else {
            get_data_strings(&block, chr_tree, coordinate)
        };
        str_array.extend(strings);
    }
    str_array
}

/// Cuts each leaf's block out of `data` and decompresses it if `is_compressed`.
///
/// `data` holds the bytes for `range`, which must cover every leaf.
pub fn decompress_data(
    data: &[u8],
    leaves: &[RTreeLeaf],
    range: &Range<u64>,
    is_compressed: bool,
) -> Result<Vec<Vec<u8>>, CodecError> {
    let mut decompressed_blocks: Vec<Vec<u8>> = Vec::new();

    for leaf in leaves {
        let begin = (leaf.data_offset - range.start) as usize;
        let end = begin + leaf.data_size as usize;
        let compressed = data[begin..end].to_vec();

        if is_compressed {
            let decompressed_block = decompress_auto(&compressed)?;
            decompressed_blocks.push(decompressed_block);
        } else {
            decompressed_blocks.push(compressed);
        };
    }
    Ok(decompressed_blocks)
}

/// Looks up the chromosome id for `coordinate` in the file's chromosome tree.
///
/// # Errors
///
/// Returns [`SearchError::InvalidRequest`] if the chromosome is not in the file.
pub fn get_bw_chromosome_id(
    index: &BigwigIndex,
    coordinate: &Coordinate,
) -> Result<u32, SearchError> {
    match index
        .chromosome_tree
        .get_chromosome_id(&coordinate.get_chromosome())
    {
        Some(id) => Ok(id),
        _ => {
            return Err(SearchError::InvalidRequest {
                reason: "Chromosome not found".to_string(),
                requested: coordinate.get_chromosome().to_string(),
            });
        }
    }
}

/// Searches a BigWig file for data overlapping each of [`SearchOptions::coordinates`].
///
/// The index is embedded in the file; it is taken from `options.bigwig_index` if present,
/// otherwise read from `file_path`, and is returned in the [`SearchResult`].
///
/// A zoom level is chosen per coordinate from the coordinate's length.  When one matches,
/// that coordinate's lines are zoom summaries; otherwise they are the raw data points.
/// Lines are grouped by coordinate in the order given.  When `include_header` is set,
/// each coordinate's lines start with a column-name line.
///
/// # Errors
///
/// Returns [`SearchError::InvalidRequest`] if a coordinate names a chromosome that is not
/// in the file.
pub async fn bigwig_search(
    store_service: &StoreService,
    options: &SearchOptions,
) -> Result<SearchResult, SearchError> {
    let mut result = SearchResult::new();
    let coordinates = options.coordinates.clone();

    let index = match &options.bigwig_index {
        Some(index) => index,
        _ => &BigwigIndex::new(store_service, &options.file_path).await?,
    };
    let is_compressed = index.header.is_compressed();
    result.bigwig_index = Some(index.clone());

    let mut ranges: Vec<Range<u64>> = Vec::with_capacity(coordinates.len());
    let mut leaves: Vec<Vec<RTreeLeaf>> = Vec::with_capacity(coordinates.len());
    let mut zoom_headers: Vec<Option<&ZoomHeader>> = Vec::with_capacity(coordinates.len());

    for coord in &coordinates {
        let zoom_header = index.get_zoom_header(&coord);
        let chr_id = get_bw_chromosome_id(&index, &coord)?;
        let index_offset = get_index_begin(index, zoom_header);
        let index_end = get_index_end(store_service, index, zoom_header).await?;
        let r_tree =
            RTree::from_file(store_service, &options.file_path, index_offset..index_end).await?;
        let leaves_instance = r_tree.get_overlapping_leaves(chr_id, &coord);
        let range = get_range_from_leaves(&leaves_instance);

        ranges.push(range);
        leaves.push(leaves_instance);
        zoom_headers.push(zoom_header);
    }

    let data = index
        .get_data(store_service, &ranges, &options.file_path)
        .await?;

    let mut decompressed_data: Vec<Vec<Vec<u8>>> = Vec::new();

    for i in 0..coordinates.len() {
        if leaves[i].is_empty() {
            decompressed_data.push(vec![]);
            continue;
        }
        let decompressed = decompress_data(&data[i], &leaves[i], &ranges[i], is_compressed)
            .map_err(|e| SearchError::CodecError {
                path: options.file_path.to_string(),
                source: e,
            })?;
        decompressed_data.push(decompressed);
    }

    let mut lines: Vec<Vec<String>> = vec![get_header_lines(
        &BigwigData::COLUMNS,
        options.include_header,
    )];
    for (i, coord) in coordinates.iter().enumerate() {
        let is_zoom = zoom_headers[i].is_some();
        let blocks = &decompressed_data[i];
        lines.push(data_to_lines(
            blocks,
            &index.chromosome_tree,
            &coord,
            is_zoom,
        ));
    }
    result.lines = lines.concat();
    Ok(result)
}

pub fn get_header_lines(columns: &[&str], include_header: bool) -> Vec<String> {
    if include_header {
        vec![
            columns
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .join("\t"),
        ]
    } else {
        Vec::new()
    }
}

fn get_index_begin(index: &BigwigIndex, zoom_header: Option<&ZoomHeader>) -> u64 {
    match zoom_header {
        Some(zoom_header) => zoom_header.index_offset,
        None => index.header.full_index_offset,
    }
}

async fn get_index_end(
    store_service: &StoreService,
    index: &BigwigIndex,
    zoom_header: Option<&ZoomHeader>,
) -> Result<u64, SearchError> {
    match zoom_header {
        Some(zoom_header) => {
            index
                .get_end_for_zoom_header(store_service, zoom_header, &index.file_path)
                .await
        }
        None => {
            index
                .get_full_index_end(store_service, &index.file_path)
                .await
        }
    }
}

/// Returns the byte range spanning every leaf's data block.
///
/// For an empty slice the range is `u64::MAX..u64::MAX`.
pub fn get_range_from_leaves(leaves: &[RTreeLeaf]) -> Range<u64> {
    let begin = leaves
        .iter()
        .map(|l| l.data_offset)
        .min()
        .unwrap_or(u64::MAX);
    let end = leaves
        .iter()
        .map(|l| l.data_offset + l.data_size)
        .max()
        .unwrap_or(u64::MAX);
    begin..end
}

use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;
use crate::bigwig::index::chr_tree::BigwigChrTree;
use crate::bigwig::index::header::BigwigHeader;
use crate::bigwig::index::r_tree::overlaps::Overlaps;
use crate::bigwig::index::total_summary::TotalSummary;
use crate::bigwig::index::zoom_header::ZoomHeader;
use crate::bigwig::zoom_data::ZoomData;
use crate::coordinate::Coordinate;
use crate::indexes::constants::{BIGWIG_HEADER_SIZE, BIGWIG_ZOOM_HEADER_SIZE};
use crate::stores::StoreService;
use std::ops::Range;

pub fn get_zoom_strings(bytes: &[u8], chr_tree: &BigwigChrTree, coord: &Coordinate) -> Vec<String> {
    let mut str_array = Vec::new();
    let mut offset = 0;
    let chromosome_id = chr_tree
        .get_chromosome_id(coord.get_chromosome())
        .unwrap_or(0);

    while offset + ZoomData::SIZE <= bytes.len() {
        if let Ok(zoom_data) =
            ZoomData::from_bytes(&bytes[offset..offset + ZoomData::SIZE], chr_tree)
        {
            if zoom_data.overlaps(
                chromosome_id,
                chromosome_id,
                coord.get_begin(),
                coord.get_end(),
            ) {
                str_array.push(format!("{}", zoom_data));
            }
            offset += ZoomData::SIZE;
        } else {
            break; // Exit if we can't parse a complete ZoomData
        }
    }

    str_array
}

pub fn get_bigwig_header_range() -> Range<u64> {
    Range {
        start: 0u64,
        end: BIGWIG_HEADER_SIZE,
    }
}

pub async fn get_bigwig_header(
    store: &StoreService,
    path_str: &str,
) -> Result<BigwigHeader, SearchError> {
    let header_range = get_bigwig_header_range();
    let header_bytes =
        store
            .get_range(path_str, header_range)
            .await
            .map_err(|e| SearchError::ReadError {
                path: path_str.to_string(),
                source: e,
            })?;
    Ok(BigwigHeader::from_bytes(&header_bytes)?)
}

pub async fn get_bigwig_detail_bytes(
    store: &StoreService,
    header: &BigwigHeader,
    path_str: &str,
) -> Result<Vec<u8>, SearchError> {
    let index_range = 0u64..header.full_data_offset + 4;
    store
        .get_range(path_str, index_range)
        .await
        .map_err(|e| SearchError::ReadError {
            path: path_str.to_string(),
            source: e,
        })
}

pub fn get_zoom_headers(
    header: &BigwigHeader,
    index_bytes: &[u8],
) -> Result<Vec<ZoomHeader>, ParsingError> {
    if header.zoom_levels == 0 {
        return Ok(Vec::new());
    }

    let mut zoom_headers = Vec::with_capacity(header.zoom_levels as usize);
    let mut offset = BIGWIG_HEADER_SIZE as usize;

    for _ in 0usize..header.zoom_levels as usize {
        if offset + BIGWIG_ZOOM_HEADER_SIZE as usize > index_bytes.len() {
            return Err(ParsingError::InsufficientBytes);
        }

        let zoom_header = ZoomHeader::from_bytes(
            &index_bytes[offset..offset + BIGWIG_ZOOM_HEADER_SIZE as usize],
        )?;
        zoom_headers.push(zoom_header);
        offset += BIGWIG_ZOOM_HEADER_SIZE as usize;
    }
    zoom_headers.sort_by_key(|z| z.reduction_level);
    zoom_headers.reverse();

    Ok(zoom_headers)
}

pub fn get_total_summary(
    header: &BigwigHeader,
    index_bytes: &[u8],
) -> Result<TotalSummary, ParsingError> {
    let start = header.total_summary_offset as usize;
    let end = header.chromosome_tree_offset as usize;

    TotalSummary::from_bytes(&index_bytes[start..end])
}

pub fn get_data_count(index_bytes: &[u8]) -> Result<u32, ParsingError> {
    if index_bytes.len() < 4 {
        return Err(ParsingError::InsufficientBytes);
    }
    let range = Range {
        start: index_bytes.len() - 4,
        end: index_bytes.len(),
    };

    let data_count = u32::from_le_bytes(index_bytes[range].try_into()?);
    Ok(data_count)
}

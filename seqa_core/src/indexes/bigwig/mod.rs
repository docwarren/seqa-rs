use std::ops::Range;
use serde::{ Serialize, Deserialize };
use thiserror::Error;
use core::array::TryFromSliceError;

use crate::api::search_options::SearchOptions;
use crate::indexes::bigwig::header::BigwigHeader;
use crate::indexes::constants::{
    BIGWIG_HEADER_SIZE,
    BIGWIG_ZOOM_HEADER_SIZE, DEFAULT_ZOOM_PIXELS,
};
use crate::stores::StoreService;
use chr_tree::BigwigChrTree;

use total_summary::TotalSummary;
use zoom_header::ZoomHeader;

pub mod chr_tree;
pub mod header;
pub mod r_tree;
pub mod total_summary;
pub mod wig_section_header;
pub mod zoom_header;
pub mod zoom_level;

#[derive(Debug, Error)]
pub enum BigwigIndexError {
    #[error("Store Error: {0}")]
    StoreError(#[from] crate::stores::error::StoreError),

    #[error("Parsing Error: {0}")]
    BigwigError(String),

    #[error("Zoom Header Error: {0}")]
    ZoomHeaderError(#[from] zoom_header::ZoomHeaderError),

    #[error("Header Error: {0}")]
    HeaderError(#[from] header::BigwigHeaderError),

    #[error("Total Summary Error: {0}")]
    TotalSummaryError(#[from] total_summary::TotalSummaryError),

    #[error("Parsing error: {0}")]
    BigwigParsingError(#[from] TryFromSliceError),

    #[error("Chromosome Tree Error: {0}")]
    ChrTreeError(#[from] chr_tree::ChrTreeError),
}

pub fn get_bigwig_header_range() -> Range<u64> {
    Range {
        start: 0u64,
        end: BIGWIG_HEADER_SIZE,
    }
}

pub async fn get_bigwig_header(store: &StoreService, path_str: &str) -> Result<BigwigHeader, BigwigIndexError> {
    let header_range = get_bigwig_header_range();
    let header_bytes = store.get_range(path_str, header_range).await?;

    Ok(BigwigHeader::from_bytes(&header_bytes)?)
}

pub async fn get_bigwig_detail_bytes(store: &StoreService, header: &BigwigHeader, path_str: &str) -> Result<Vec<u8>, BigwigIndexError> {
    let index_range = 0u64..header.full_data_offset as u64 + 4;
    Ok(store.get_range(path_str, index_range).await?)
}

pub fn get_zoom_headers(header: &BigwigHeader, index_bytes: &[u8]) -> Result<Vec<ZoomHeader>, BigwigIndexError> {
    if header.zoom_levels == 0 {
        return Ok(Vec::new());
    }

    let mut zoom_headers = Vec::with_capacity(header.zoom_levels as usize);
    let mut offset = BIGWIG_HEADER_SIZE as usize;

    for _ in 0usize..header.zoom_levels as usize {
        if offset + BIGWIG_ZOOM_HEADER_SIZE as usize > index_bytes.len() {
            return Err(BigwigIndexError::BigwigError("Not enough bytes for a complete ZoomHeader".to_string()));
        }

        let zoom_header = ZoomHeader::from_bytes(&index_bytes[offset..offset + BIGWIG_ZOOM_HEADER_SIZE as usize])?;
        zoom_headers.push(zoom_header);
        offset += BIGWIG_ZOOM_HEADER_SIZE as usize;
    }
    zoom_headers.sort_by_key(|z| z.reduction_level);
    zoom_headers.reverse();

    Ok(zoom_headers)
}

pub fn get_total_summary(header: &BigwigHeader, index_bytes: &[u8]) -> Result<TotalSummary, BigwigIndexError> {
    let start = header.total_summary_offset as usize;
    let end = header.chromosome_tree_offset as usize;

    Ok(TotalSummary::from_bytes(&index_bytes[start..end])?)
}

fn get_data_count(index_bytes: &[u8]) -> Result<u32, BigwigIndexError> {
    if index_bytes.len() < 4 {
        return Err(BigwigIndexError::BigwigError("Not enough bytes for data count".to_string()));
    }
    let range = Range {
        start: index_bytes.len() - 4,
        end: index_bytes.len(),
    };

    let data_count = u32::from_le_bytes(index_bytes[range].try_into()?);
    Ok(data_count)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BigwigIndex {
    pub header: header::BigwigHeader,
    pub zoom_headers: Vec<ZoomHeader>,
    pub total_summary: TotalSummary,
    pub chromosome_tree: BigwigChrTree,
    pub data_count: u32,
    pub file_path: String,
}

impl BigwigIndex {
    pub async fn new(
        store: &StoreService,
        file_path: &str,
    ) -> Result<BigwigIndex, BigwigIndexError> {
        let bigwig_header =  get_bigwig_header(store, file_path).await?;
        let detail_bytes =  get_bigwig_detail_bytes(store, &bigwig_header, file_path).await?;
        let zoom_headers =  get_zoom_headers(&bigwig_header, &detail_bytes)?;
        let total_summary =  get_total_summary(&bigwig_header, &detail_bytes)?;
        let chr_tree =  BigwigChrTree::from_bytes(&detail_bytes, &bigwig_header)?;
        let data_count =  get_data_count(&detail_bytes)?;

        
        Ok(BigwigIndex {
            header: bigwig_header,
            zoom_headers,
            total_summary: total_summary,
            chromosome_tree: chr_tree,
            data_count: data_count,
            file_path: file_path.to_string(),
        })
    }

    pub fn get_zoom_header(&self, options: &SearchOptions) -> Option<&ZoomHeader> {
        let reduction_level = (options.end as f32 - options.begin as f32) / DEFAULT_ZOOM_PIXELS;
        self.zoom_headers.iter().find(|zoom| zoom.matches(reduction_level))
    }

    pub async fn get_end_for_zoom_header(
        &self,
        store: &StoreService,
        zoom_header: &ZoomHeader,
        path_str: &str,
    ) -> Result<u64, BigwigIndexError> {
        match self.get_next_zoom_header(zoom_header) {
            Some(next) => Ok(next.index_offset as u64),
            None => Ok(store.get_file_size(path_str).await?),
        }
    }

    pub fn get_next_zoom_header(&self, zoom_header: &ZoomHeader) -> Option<ZoomHeader> {
        let mut zoom_headers = self.zoom_headers.clone();
        zoom_headers.sort_by_key(|z| z.index_offset);
        let index = zoom_headers.iter().position(|z| z.index_offset == zoom_header.index_offset);
        match index {
            Some(i) => zoom_headers.get(i + 1).cloned(),
            _ => None,
        }
    }

    pub async fn get_full_index_end(
        &self,
        store: &StoreService,
        path_str: &str,
    ) -> Result<u64, BigwigIndexError> {
        match self.zoom_headers.first() {
            Some(zoom) => Ok(zoom.index_offset as u64),
            None => Ok(store.get_file_size(path_str).await?),
        }
    }

    pub async fn get_data(
        &self,
        store: &StoreService,
        range: &Range<u64>,
        path_str: &str,
    ) -> Result<Vec<u8>, BigwigIndexError> {
        Ok(store.get_range(path_str, range.clone()).await?)
    }
}

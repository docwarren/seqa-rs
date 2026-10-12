use serde::{Deserialize, Serialize};
use std::ops::Range;

use crate::api::search_options::SearchOptions;
use crate::bigwig::index::chr_tree::BigwigChrTree;
use crate::bigwig::index::header;
use crate::bigwig::index::total_summary::TotalSummary;
use crate::bigwig::index::util::{get_bigwig_detail_bytes, get_bigwig_header, get_data_count, get_total_summary, get_zoom_headers};
use crate::bigwig::index::zoom_header::ZoomHeader;
use crate::indexes::constants::DEFAULT_ZOOM_PIXELS;
use crate::stores::StoreService;
use crate::api::search_error::SearchError;
use crate::coordinate::Coordinate;

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
    ) -> Result<BigwigIndex, SearchError> {
        let bigwig_header =  get_bigwig_header(store, file_path).await?;
        let detail_bytes =  get_bigwig_detail_bytes(store, &bigwig_header, file_path).await?;
        let zoom_headers =  get_zoom_headers(&bigwig_header, &detail_bytes)
            .map_err(|e| SearchError::ParseError {
                path: file_path.to_string(),
                source: e
            }
        )?;
        let total_summary =  get_total_summary(&bigwig_header, &detail_bytes)
            .map_err(|e| SearchError::ParseError {
                path: file_path.to_string(),
                source: e
            })?;
        let chromosome_tree =  BigwigChrTree::from_bytes(&detail_bytes, &bigwig_header)
            .map_err(|e| SearchError::ParseError {
                path: file_path.to_string(),
                source: e
            })?;
        let data_count =  get_data_count(&detail_bytes)
            .map_err(|e| SearchError::ParseError {
                path: file_path.to_string(),
                source: e
            })?;

        Ok(BigwigIndex {
            header: bigwig_header,
            zoom_headers,
            total_summary,
            chromosome_tree,
            data_count,
            file_path: file_path.to_string(),
        })
    }

    pub fn get_zoom_header(&self, coords: &Coordinate) -> Option<&ZoomHeader> {
        let reduction_level = (coords.get_length() as f32) / DEFAULT_ZOOM_PIXELS;
        self.zoom_headers.iter().find(|zoom| zoom.matches(reduction_level))
    }

    pub async fn get_end_for_zoom_header(
        &self,
        store: &StoreService,
        zoom_header: &ZoomHeader,
        path_str: &str,
    ) -> Result<u64, SearchError> {
        match self.get_next_zoom_header(zoom_header) {
            Some(next) => Ok(next.index_offset),
            None => Ok(store.get_file_size(path_str).await
                .map_err(|e| SearchError::ReadError {
                    path: path_str.to_string(),
                    source: e
                })?),
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
    ) -> Result<u64, SearchError> {
        match self.zoom_headers.first() {
            Some(zoom) => Ok(zoom.index_offset),
            None => Ok(store.get_file_size(path_str).await.map_err(|e| SearchError::ReadError {
                path: path_str.to_string(),
                source:e
            })?),
        }
    }

    pub async fn get_data(
        &self,
        store: &StoreService,
        range: &Range<u64>,
        path_str: &str,
    ) -> Result<Vec<u8>, SearchError> {
        store.get_range(path_str, range.clone()).await.map_err(|e| SearchError::ReadError {
            path: path_str.to_string(),
            source: e
        })
    }
}

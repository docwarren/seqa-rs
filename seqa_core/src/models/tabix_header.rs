use std::ops::Range;
use serde::{ Serialize, Deserialize};
use thiserror::Error;

use crate::codecs::bgzip;
use crate::indexes::constants::MAX_BLOCK_SIZE;
use crate::indexes::virtual_offset::VirtualOffset;
use crate::stores::error::StoreError;
use crate::stores::StoreService;

#[derive(Debug, Error)]
pub enum TabixHeaderError {
    #[error("Failed to read Tabix header file: {0}")]
    ReadError(String),

    #[error("StoreError: {0}")]
    StoreError(#[from] StoreError),

    #[error("BgZip Error: {0}")]
    BgZipError(#[from] bgzip::BgZipError),

    #[error("Parsing Error: {0}")]
    ParsingError(#[from] core::array::TryFromSliceError),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TabixHeader {
    lines: Vec<String>,
}

impl TabixHeader {
    pub fn new() -> Self {
        TabixHeader { lines: Vec::new() }
    }

    pub async fn from_file(
        store: &StoreService,
        file_path: &str,
        first_vp: VirtualOffset,
    ) -> Result<Self, TabixHeaderError> {
        let compressed_bytes = store
            .get_range(file_path, Range {
                start: 0u64,
                end: first_vp.block_offset as u64 + MAX_BLOCK_SIZE,
            })
            .await?;

        let block_sizes = bgzip::from_bytes(&compressed_bytes)?;
        let bytes = bgzip::decompress(&block_sizes, &compressed_bytes)?;
        let header_str = String::from_utf8_lossy(&bytes);

        let lines = header_str.lines()
            .filter(|line| {
                line.starts_with('#')
            })
            .map(|line| line.to_string()).collect();

        Ok(TabixHeader { lines })
    }

    pub fn to_lines(&self) -> Vec<String> {
        self.lines.clone()
    }
}
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::header_line::HeaderLine;
use super::header_utils::read_magic;
use super::reference::BamReference;
use crate::codecs::bgzip;
use crate::indexes::constants::MAX_BLOCK_SIZE;
use crate::indexes::virtual_offset::VirtualOffset;
use crate::stores::error::StoreError;
use crate::stores::StoreService;
use std::ops::Range;

#[derive(Debug, Error)]
pub enum BamHeaderError {
    #[error("Invalid BAM header line: {0}")]
    InvalidHeaderLine(String),

    #[error("StoreError: {0}")]
    StoreError(#[from] StoreError),

    #[error("BgZip Error: {0}")]
    BgZipError(#[from] bgzip::BgZipError),

    #[error("Parsing Error: {0}")]
    ParsingError(#[from] core::array::TryFromSliceError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BamHeader {
    pub header_lines: Vec<HeaderLine>,
    pub references: Vec<BamReference>,
}

impl BamHeader {
    pub fn new() -> Self {
        BamHeader {
            header_lines: Vec::new(),
            references: Vec::new(),
        }
    }

    pub async fn from_file(
        store: &StoreService,
        file_path: &str,
        first_vp: VirtualOffset,
    ) -> Result<Self, BamHeaderError> {
        let compressed_bytes = store
            .get_range(file_path, Range {
                start: 0u64,
                end: first_vp.block_offset as u64 + MAX_BLOCK_SIZE,
            })
            .await?;

        let block_sizes = bgzip::from_bytes(&compressed_bytes)?;
        let bytes = bgzip::decompress(&block_sizes, &compressed_bytes)?;

        let mut i = 0;
        let (_, l_text) = read_magic(&bytes).await?;

        i += 8;

        let header_lines = BamHeader::text_header_from_bytes(bytes[i..i + l_text as usize].to_vec())?;

        i += l_text as usize;

        let n_ref = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let mut references = Vec::with_capacity(n_ref as usize);

        for _ in 0..n_ref {
            let l_name = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
            i += 4;

            let ref_name_bytes = &bytes[i..i + l_name as usize];
            let mut ref_name = String::from_utf8_lossy(ref_name_bytes).to_string();
            ref_name = ref_name.trim_end_matches('\0').to_string();

            i += l_name as usize;

            let ref_length = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
            i += 4;

            references.push(BamReference {
                name: ref_name,
                length: ref_length,
            });
        }

        Ok(BamHeader {
            header_lines,
            references,
        })
    }

    pub fn text_header_from_bytes(bytes: Vec<u8>) -> Result<Vec<HeaderLine>, BamHeaderError> {
        let header_str = String::from_utf8_lossy(&bytes);

        let mut header_lines = Vec::new();
        for line in header_str.lines() {
            let header_line = HeaderLine::from_line(line.to_string())?;
            header_lines.push(header_line);
        }

        Ok(header_lines)
    }
}

impl BamHeader {
    pub fn get_chromosome_index_by_name(&self, name: &str) -> Option<usize> {
        crate::genome::chromosome_aliases(name)
            .iter()
            .find_map(|alias| self.references.iter().position(|r| &r.name == alias))
    }

    pub fn get_chromosome_name_by_index(&self, index: usize) -> Option<String> {
        if index < self.references.len() {
            Some(self.references[index].name.clone())
        } else {
            None
        }
    }

    pub fn to_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for header_line in &self.header_lines {
            lines.push(format!("{}", header_line));
        }
        lines
    }
}

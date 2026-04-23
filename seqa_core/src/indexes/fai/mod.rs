use std::collections::HashMap;
use std::ops::Range;
use std::string::FromUtf8Error;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use core::array::TryFromSliceError;

use crate::stores::error::StoreError;
use crate::stores::StoreService;
use crate::api::search_options::SearchOptions;

#[derive(Debug, Error)]
pub enum FaiIndexError {
    #[error("Failed to read FAI index file: {0}")]
    ReadError(String),

    #[error("Store Error: {0}")]
    StoreError(#[from] StoreError),

    #[error("Failed to parse FAI index file: {0}")]
    ParseError(#[from] TryFromSliceError),

    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] FromUtf8Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contig {
    pub name: String,
    pub length: u64,
    pub bases_per_line: u32,
    pub bytes_per_line: u32,
    pub offset: u64,
    pub qual_offset: u64
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaiIndex {
    pub contigs: HashMap<String, Contig>,
}

impl FaiIndex {
    pub fn new() -> Self {
        FaiIndex {
            contigs: HashMap::new(),
        }
    }

    pub async fn from_file(
        store_service: &StoreService,
        idx_path: &str,
    ) -> Result<Self, FaiIndexError> {
        let bytes = store_service.get_object(idx_path).await?;

        Ok(FaiIndex::from_bytes(bytes)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, FaiIndexError> {
        let mut fai_index = FaiIndex::new();
        let lines = String::from_utf8(bytes)?;

        for line in lines.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue; // Skip empty lines and comments
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 5 {
                continue; // Invalid line format
            }
            let contig = Contig {
                name: parts[0].to_string(),
                length: parts[1].parse().unwrap_or(0),
                offset: parts[2].parse().unwrap_or(0),
                bases_per_line: parts[3].parse().unwrap_or(0),
                bytes_per_line: parts[4].parse().unwrap_or(0),
                qual_offset: if parts.len() > 5 {
                    parts[5].parse().unwrap_or(0)
                } else {
                    0 // Default value if not provided
                },
            };
            fai_index.contigs.insert(contig.name.clone(), contig);
        }
        Ok(fai_index)
    }

    pub fn get_offsets(&self, options: &SearchOptions) -> Result<Range<u64>, FaiIndexError> {
        let contig = self.contigs.get(&options.chromosome).ok_or(FaiIndexError::ReadError(format!("Contig not found: {}", options.chromosome)))?;
        if options.begin < 1 || options.end > contig.length as u32 {
            return Err(FaiIndexError::ReadError(format!("Invalid coordinates: {}-{}", options.begin, options.end)));
        }
        let start = Self::get_offset(options.begin, contig);
        let end = Self::get_offset(options.end, contig);

        Ok(start..end)
    }

    pub fn get_offset(position: u32, contig: &Contig) -> u64 {
        let line_number = if position == 1 {
            0
        } else {
            (position as u64 - 1) / contig.bases_per_line as u64
        };
        let line_offset = line_number * contig.bytes_per_line as u64;
        let base_offset = if position == 1 {
            0
        } else {
            (position as u64 - 1) % contig.bases_per_line as u64
        };
        contig.offset + line_offset + base_offset
    }
}
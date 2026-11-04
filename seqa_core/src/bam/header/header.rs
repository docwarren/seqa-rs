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

use crate::api::parsing_error::ParsingError;
use crate::api::search_error::SearchError;
use crate::bam::header::header_line::HeaderLine;
use crate::bam::header::header_utils::read_magic;
use crate::bam::header::reference::BamReference;
use crate::codecs::bgzip;
use crate::indexes::constants::MAX_BLOCK_SIZE;
use crate::indexes::virtual_offset::VirtualOffset;
use crate::stores::StoreService;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BamHeader {
    pub header_lines: Vec<HeaderLine>,
    pub references: Vec<BamReference>,
}

fn parse_header_lines(bytes: &[u8]) -> Result<BamHeader, ParsingError> {
    let mut i = 0;

    let (_, l_text) = read_magic(bytes)?;
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
    ) -> Result<Self, SearchError> {
        let compressed_bytes = store
            .get_range(
                file_path,
                Range {
                    start: 0u64,
                    end: first_vp.block_offset + MAX_BLOCK_SIZE,
                },
            )
            .await
            .map_err(|e| SearchError::ReadError {
                path: file_path.to_string(),
                source: e,
            })?;

        let block_sizes =
            bgzip::from_bytes(&compressed_bytes).map_err(|e| SearchError::CodecError {
                path: file_path.to_string(),
                source: e,
            })?;

        let bytes = bgzip::decompress(&block_sizes, &compressed_bytes).map_err(|e| {
            SearchError::CodecError {
                path: file_path.to_string(),
                source: e,
            }
        })?;

        parse_header_lines(&bytes).map_err(|e| SearchError::ParseError {
            path: file_path.to_string(),
            source: e,
        })
    }

    pub fn text_header_from_bytes(bytes: Vec<u8>) -> Result<Vec<HeaderLine>, ParsingError> {
        let header_str = String::from_utf8_lossy(&bytes);
        let mut header_lines = Vec::new();

        for line in header_str.lines() {
            let header_line = HeaderLine::from_line(line)?;
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

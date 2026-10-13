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
use crate::bam::cigar::Cigar;
use crate::bam::header::header::BamHeader;
use crate::bam::qual::Qual;
use crate::bam::seq::Seq;
use crate::bam::tags::Tags;
use crate::coordinate::Coordinate;
use crate::traits::from_bytes::FromBytes;
use crate::{models::coordinate_system::CoordinateSystem, traits::feature::Feature};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Serialize, Deserialize)]
pub struct Read {
    block_size: u32,
    pub ref_id: i32,
    pub pos: i32,
    l_read_name: u8,
    mapq: u8,
    bin: u16,
    n_cigar_op: u16,
    flag: u16,
    pub l_seq: u32,
    next_ref_id: i32,
    next_pos: i32,
    t_len: i32,
    read_name: Vec<u8>,
    cigar: Cigar,
    seq: Seq,
    qual: Qual,
    tags: Tags,
}

impl Read {
    /// Creates a mock reference string from an MD tag.
    /// Matches are represented as '-', mismatches as their base letter.
    /// e.g., MD "7^C5G4" -> "-------C-----G----"
    pub fn init_mock_ref(md: &str) -> String {
        let mut ref_string = String::new();
        let mut num_buffer = String::new();

        for ch in md.chars() {
            if ch.is_ascii_digit() {
                num_buffer.push(ch);
            } else {
                if !num_buffer.is_empty() {
                    let count: usize = num_buffer.parse().unwrap_or(0);
                    ref_string.push_str(&"-".repeat(count));
                    num_buffer.clear();
                }
                if ch != '^' {
                    ref_string.push(ch);
                }
            }
        }
        // Handle trailing number
        if !num_buffer.is_empty() {
            let count: usize = num_buffer.parse().unwrap_or(0);
            ref_string.push_str(&"-".repeat(count));
        }
        ref_string
    }

    /// Adds soft clips and insertions to the mock reference based on cigar operations.
    fn init_clipped_ref(&self, mock_ref: &str) -> String {
        let ops = self.cigar.get_operations();
        if ops.is_empty() {
            return mock_ref.to_string();
        }

        let mut result = String::new();
        let mut ref_index = 0;
        let mock_ref_bytes = mock_ref.as_bytes();

        for op in ops {
            let op_char = op.get_op_char();
            let len = op.get_length() as usize;

            match op_char {
                'S' => {
                    result.push_str(&".".repeat(len));
                }
                'I' => {
                    result.push_str(&"I".repeat(len));
                }
                'H' | 'N' => {
                    // Hard clips and skipped regions don't appear in mockRef
                }
                _ => {
                    // M, D, =, X consume reference positions in mockRef
                    let end = (ref_index + len).min(mock_ref_bytes.len());
                    if ref_index < mock_ref_bytes.len() {
                        result.push_str(&mock_ref[ref_index..end]);
                    }
                    ref_index = end;
                }
            }
        }
        result
    }

    /// Creates an enhanced read string with deletions marked as '-'.
    fn init_enhanced_read(&self) -> String {
        let mut new_seq = String::new();
        let mut seq_index = 0;
        let sequence = self.seq.as_string();
        let seq_bytes = sequence.as_bytes();

        for op in self.cigar.get_operations() {
            let op_char = op.get_op_char();
            let len = op.get_length() as usize;

            match op_char {
                'D' => {
                    new_seq.push_str(&"-".repeat(len));
                }
                'H' | 'N' => {
                    // Hard clips and skipped regions don't consume sequence
                }
                _ => {
                    let end = (seq_index + len).min(seq_bytes.len());
                    if seq_index < seq_bytes.len() {
                        new_seq.push_str(&sequence[seq_index..end]);
                    }
                    seq_index = end;
                }
            }
        }
        new_seq
    }

    /// Creates a merged cigar string from the clipped reference and enhanced read.
    pub fn init_merged_cigar_string(ref_string: &str, read_string: &str) -> String {
        let mut merged_cigar = String::new();

        let ref_chars: Vec<char> = ref_string.chars().collect();
        let rd_chars: Vec<char> = read_string.chars().collect();

        let mut op_count = 0usize;
        let mut current_op: Option<char> = None;
        let mut current_read_base: Option<char> = None;

        let len = ref_chars.len().max(rd_chars.len()) + 1;

        for i in 0..len {
            let ref_char = ref_chars.get(i).copied();
            let rd_char = rd_chars.get(i).copied();

            // Determine operation type
            let operation = match (ref_char, rd_char) {
                (Some(_), Some('-')) => Some('D'),
                (Some(r), _) if matches!(r, 'A' | 'C' | 'G' | 'T') => Some('X'),
                (Some(r), _) => Some(r),
                _ => None,
            };

            // Initialize current_op and current_read_base on first iteration
            if current_op.is_none() {
                current_op = operation;
            }
            if current_read_base.is_none() {
                current_read_base = rd_char;
            }

            if operation == current_op {
                op_count += 1;
            } else {
                // Output the previous operation
                if let Some(op) = current_op {
                    match op {
                        '.' => {
                            merged_cigar.push_str(&format!("{}S", op_count));
                        }
                        '-' => {
                            merged_cigar.push_str(&format!("{}M", op_count));
                        }
                        'I' => {
                            merged_cigar.push_str(&format!("{}I", op_count));
                        }
                        'D' => {
                            // Deletion: include ref bases
                            let start = i.saturating_sub(op_count);
                            let bases: String = ref_chars[start..i].iter().collect();
                            merged_cigar.push_str(&format!("{}D{}", op_count, bases));
                        }
                        'X' => {
                            // Mismatch: include read bases
                            let start = i.saturating_sub(op_count);
                            let bases: String = rd_chars[start..i].iter().collect();
                            merged_cigar.push_str(&format!("{}X{}", op_count, bases));
                        }
                        _ => {}
                    }
                }
                op_count = 1;
                current_op = operation;
                current_read_base = rd_char;
            }
        }

        merged_cigar
    }

    /// Creates a merged cigar string that includes mismatch and deletion base information.
    pub fn get_merged_cigar(&self) -> String {
        let md = match self.tags.get_value("MD") {
            Some(md) => md,
            None => return self.cigar.to_string(),
        };

        let mock_ref = Self::init_mock_ref(&md);
        let clipped_ref = self.init_clipped_ref(&mock_ref);
        let enhanced_read = self.init_enhanced_read();
        Self::init_merged_cigar_string(&clipped_ref, &enhanced_read)
    }

    pub fn get_ref_name(&self, bam_header: &BamHeader) -> Option<String> {
        bam_header.get_chromosome_name_by_index(self.ref_id as usize)
    }

    pub fn get_next_ref_name(&self, bam_header: &BamHeader) -> Option<String> {
        if self.next_ref_id == self.ref_id {
            Some("=".to_string())
        } else {
            bam_header.get_chromosome_name_by_index(self.next_ref_id as usize)
        }
    }
}

impl FromBytes for Read {
    type Error = ParsingError;
    fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), ParsingError> {
        let mut i = 0;
        if bytes.len() < i + 4 {
            return Err(ParsingError::InsufficientBytes);
        }

        let block_size = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let block_end = i + block_size as usize;

        if block_end > bytes.len() {
            return Err(ParsingError::InsufficientBytes);
        }

        let ref_id = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let pos = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let l_read_name = bytes[i];
        i += 1;

        let mapq = bytes[i];
        i += 1;

        let bin = u16::from_le_bytes(bytes[i..i + 2].try_into()?);
        i += 2;

        let n_cigar_op = u16::from_le_bytes(bytes[i..i + 2].try_into()?);
        i += 2;

        let flag = u16::from_le_bytes(bytes[i..i + 2].try_into()?);
        i += 2;

        let l_seq = u32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let next_ref_id = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let next_pos = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let t_len = i32::from_le_bytes(bytes[i..i + 4].try_into()?);
        i += 4;

        let read_name = bytes[i..i + l_read_name as usize].to_vec();
        i += l_read_name as usize;

        let cigar_end = i + n_cigar_op as usize * 4;
        let cigar = Cigar::from_bytes(&bytes[i..cigar_end])?.0;
        i += n_cigar_op as usize * 4;

        let seq_end = i + ((l_seq as usize + 1) / 2);
        let seq = Seq::from_bytes(&bytes[i..seq_end])?.0;
        i += (l_seq as usize + 1) / 2;

        let qual = Qual::from_bytes(&bytes[i..i + l_seq as usize])?.0;
        i += l_seq as usize;

        let tags = Tags::from_bytes(&bytes[i..block_end])?.0;
        i = block_end;

        Ok((
            Read {
                block_size,
                ref_id,
                pos,
                l_read_name,
                mapq,
                bin,
                n_cigar_op,
                flag,
                l_seq,
                next_ref_id,
                next_pos,
                t_len,
                read_name,
                cigar,
                seq,
                qual,
                tags,
            },
            i,
        ))
    }
}

impl Read {
    fn get_read_name(&self) -> String {
        String::from_utf8_lossy(&self.read_name)
            .trim_matches('\0')
            .to_string()
    }

    /// Formats the read as a SAM line with the specified cigar format.
    pub fn to_sam_string(&self, use_merged_cigar: bool, bam_header: &BamHeader) -> String {
        let chr = self
            .get_ref_name(bam_header)
            .unwrap_or_else(|| format!("{}", self.ref_id));

        let next_chr = self
            .get_next_ref_name(bam_header)
            .unwrap_or_else(|| format!("{}", self.next_ref_id));

        let cigar_str = if use_merged_cigar {
            self.get_merged_cigar()
        } else {
            format!("{}", self.cigar)
        };

        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.get_read_name(),
            self.flag,
            chr,
            self.pos + 1,
            self.mapq,
            cigar_str,
            next_chr,
            self.next_pos + 1,
            self.t_len,
            self.seq,
            self.qual,
            self.tags
        )
    }

    pub fn get_begin(&self) -> u32 {
        self.pos as u32
    }

    pub fn get_end(&self) -> u32 {
        let cigar_len = self.cigar.get_reference_length();
        if cigar_len == 0 {
            // Unmapped reads: use sequence length or treat as 1-base point
            self.get_begin() + self.l_seq.max(1)
        } else {
            self.get_begin() + cigar_len
        }
    }
}

impl Display for Read {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.read_name,
            self.flag,
            self.ref_id,
            self.pos + 1,
            self.mapq,
            self.cigar,
            self.next_ref_id,
            self.next_pos + 1,
            self.t_len,
            self.seq,
            self.qual,
            self.tags
        )?;
        Ok(())
    }
}

impl Feature for Read {
    fn get_id(&self) -> String {
        String::from_utf8(self.read_name.clone()).unwrap()
    }

    fn coordinate(&self) -> Coordinate {
        Coordinate {
            chromosome: self.ref_id.to_string(),
            begin: self.get_begin(),
            end: self.get_end(),
            system: CoordinateSystem::ZeroBasedHalfOpen,
        }
    }
}

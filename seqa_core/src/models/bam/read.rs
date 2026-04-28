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

use std::fmt::Display;
use crate::{models::{bam_header::header::BamHeader, coordinates::CoordinateSystem}, traits::feature::Feature};
use super::{cigar::Cigar, seq::Seq, qual::Qual, tags::Tags};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Read {
    block_size: u32,
    ref_id: i32,
    ref_name: String, // This will be set later based on the ref_id
    pub pos: i32,
    l_read_name: u8,
    mapq: u8,
    bin: u16,
    n_cigar_op: u16,
    flag: u16,
    pub l_seq: u32,
    next_ref_id: i32,
    next_ref_str: String, // This will be set later based on the next_ref_id
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
        let sequence = self.seq.to_string();
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

    pub fn from_bytes(bytes: &Vec<u8>, mut i: usize, bam_header: &BamHeader) -> Result<(Self, usize), String> {
        if bytes.len() < i + 4 {
            return Err("Not enough bytes for block size".into());
        }

        let block_size = u32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid block size".to_string())?);
        i += 4;

        let block_end = i + block_size as usize;

        if block_end > bytes.len() {
            return Err("Not enough bytes for a complete read".into());
        }

        let ref_id = i32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid ref_id".to_string())?);
        i += 4;

        let ref_name = bam_header.get_chromosome_name_by_index(ref_id as usize)
            .unwrap_or_else(|| format!("chr{}", ref_id));

        let pos = i32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid pos".to_string())?);
        i += 4;

        let l_read_name = bytes[i];
        i += 1;

        let mapq = bytes[i];
        i += 1;

        let bin = u16::from_le_bytes(bytes[i..i + 2].try_into().map_err(|_| "Invalid bin".to_string())?);
        i += 2;

        let n_cigar_op = u16::from_le_bytes(bytes[i..i + 2].try_into().map_err(|_| "Invalid n_cigar_op".to_string())?);
        i += 2;

        let flag = u16::from_le_bytes(bytes[i..i + 2].try_into().map_err(|_| "Invalid flag".to_string())?);
        i += 2;

        let l_seq = u32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid l_seq".to_string())?);
        i += 4;

        let next_ref_id = i32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid next_ref_id".to_string())?);
        i += 4;

        let next_ref_str = if next_ref_id == ref_id {
            format!("=")
        }
        else {
            bam_header.get_chromosome_name_by_index(next_ref_id as usize)
            .unwrap_or_else(|| format!("*"))
        };

        let next_pos = i32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid next_pos".to_string())?);
        i += 4;

        let t_len = i32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid t_len".to_string())?);
        i += 4;

        let read_name = bytes[i..i + l_read_name as usize].to_vec();
        i += l_read_name as usize;

        let cigar = Cigar::from_bytes(bytes[i..i + (n_cigar_op as usize * 4)].to_vec())?;
        i += n_cigar_op as usize * 4;

        let seq = Seq::from_bytes(bytes[i..i + ((l_seq as usize + 1) / 2)].to_vec());
        i += (l_seq as usize + 1) / 2;

        let qual = Qual::from_bytes(bytes[i..i + l_seq as usize].to_vec());
        i += l_seq as usize;

        let tags = Tags::from_bytes(bytes[i..block_end].to_vec());
        i = block_end;

        Ok((
            Read {
                block_size,
                ref_id,
                ref_name,
                pos,
                l_read_name,
                mapq,
                bin,
                n_cigar_op,
                flag,
                l_seq,
                next_ref_id,
                next_ref_str,
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
    /// Formats the read as a SAM line with the specified cigar format.
    pub fn to_sam_string(&self, use_merged_cigar: bool) -> String {
        let read_name = String::from_utf8_lossy(&self.read_name).trim_matches('\0').to_string();
        let cigar_str = if self.cigar.is_empty() {
            "*".to_string()
        } else if use_merged_cigar {
            self.get_merged_cigar()
        } else {
            self.cigar.to_string()
        };

        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            read_name,
            self.flag,
            self.ref_name,
            self.pos + 1,
            self.mapq,
            cigar_str,
            self.next_ref_str,
            self.next_pos + 1,
            self.t_len,
            self.seq,
            self.qual,
            self.tags
        )
    }
}

impl Display for Read {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sam_string(false))
    }
}

impl Feature for Read {
    fn coordinate_system(&self) -> CoordinateSystem {
        CoordinateSystem::ZeroBasedHalfOpen
    }

    fn get_chromosome(&self) -> String {
        self.ref_name.to_owned()
    }

    fn get_begin(&self) -> u32 {
        self.pos as u32
    }

    fn get_end(&self) -> u32 {
        let cigar_len = self.cigar.get_reference_length();
        if cigar_len == 0 {
            // Unmapped reads: use sequence length or treat as 1-base point
            self.get_begin() + self.l_seq.max(1)
        } else {
            self.get_begin() + cigar_len
        }
    }

    fn get_length(&self) -> u32 {
        self.cigar.get_read_length()
    }

    fn get_id(&self) -> String {
        String::from_utf8(self.read_name.clone()).unwrap()
    }
}

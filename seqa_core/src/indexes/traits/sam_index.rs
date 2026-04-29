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

use crate::{api::search_options::SearchOptions, indexes::{
    bai::chr_idx::ChrIdx,
    chunk::Chunk,
    chunk_util::{filter_chunks, optimize},
    constants::LINEAR_BIN_SIZE,
    virtual_offset::VirtualOffset,
}};

pub trait SamIndex {
    fn get_chunks(&self, chr_idx: &ChrIdx, bins: Vec<u32>) -> Vec<Chunk> {
        let mut chunks = Vec::new();

        for bin_no in bins {
            if let Some(bin) = chr_idx.bins.get(&bin_no) {
                for chunk in bin.chunks.clone() {
                    chunks.push(chunk);
                }
            }
        }
        chunks
    }

    fn get_linear_bin(&self, chr_idx: &ChrIdx, begin: u32) -> Option<VirtualOffset> {
        let bin_i = begin / LINEAR_BIN_SIZE;
        if bin_i as usize >= chr_idx.intervals.len() {
            return None;
        }
        let first_record = VirtualOffset::new(chr_idx.intervals[bin_i as usize]);
        Some(first_record)
    }

    fn get_optimized_chunks(&self, chr_idx: &ChrIdx, bins: Vec<u32>, options: &SearchOptions) -> Vec<Chunk> {
        let chunks = self.get_chunks(chr_idx, bins);
        let linear_bin_begin = self.get_linear_bin(chr_idx, options.begin);

        let chunks = match linear_bin_begin {
            None => chunks, // If no linear bin is found, return the original chunks
            Some(linear_bin_begin) => { 
                filter_chunks(chunks, linear_bin_begin)
            }
        };
        optimize(chunks)
    }
}

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

use crate::indexes::constants::MAX_BLOCK_SIZE;

use super::{chunk::Chunk, virtual_offset::VirtualOffset};

pub fn optimize(chunks: Vec<Chunk>) -> Vec<Chunk> {

    if chunks.len() < 2 {
        return chunks; // No need to optimize if there's only one chunk
    }
    // Sort the chunks by their start position
    let mut sorted_chunks = chunks.clone();
    sorted_chunks.sort_by_key(|chunk| chunk.begin_vp);

    // Merge overlapping chunks
    let mut optimized_chunks = Vec::new();
    let mut current_chunk = sorted_chunks[0].clone();

    for chunk in &sorted_chunks[1..] {
        if chunk.begin_vp.block_offset <= current_chunk.end_vp.block_offset + MAX_BLOCK_SIZE {
            // Overlapping chunks, merge them
            current_chunk.end_vp = current_chunk.end_vp.max(chunk.end_vp);
        } else {
            // No overlap, push the current chunk and start a new one
            optimized_chunks.push(current_chunk);
            current_chunk = chunk.clone();
        }
    }
    optimized_chunks.push(current_chunk); // Push the last chunk

    optimized_chunks
}

pub fn filter_chunks(chunks: Vec<Chunk>, min_offset: VirtualOffset) -> Vec<Chunk> {
    chunks.into_iter().filter(|chunk| chunk.end_vp >= min_offset).collect()
}
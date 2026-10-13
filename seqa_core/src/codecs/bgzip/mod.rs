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

pub mod bgzip_block;

use super::gzip::gzip_decompress;
use crate::codecs::codec_error::CodecError;
use bgzip_block::BgZipBlock;

/// Reads BGZIP blocks from a byte vector
/// Returns a vector of block sizes.
pub fn from_bytes(bytes: &[u8]) -> Result<Vec<usize>, CodecError> {
    let mut i = 0;
    let mut blocks = Vec::with_capacity(bytes.len() / (16 * 1024) + 1);
    while i < bytes.len() {
        let block = BgZipBlock::from_bytes(bytes, i);
        match block {
            Ok(block) => {
                let size = block.sub_block.bsize as usize + 1;
                i += size;
                blocks.push(size);
            }
            Err(_) => break,
        }
    }
    Ok(blocks)
}

/// Decompresses BGZIP blocks from a byte vector
/// Takes a vector of block sizes and a byte slice containing the compressed data.
pub fn decompress(block_sizes: &[usize], bytes: &[u8]) -> Result<Vec<u8>, CodecError> {
    let mut i = 0;
    let mut result = Vec::with_capacity(block_sizes.len());
    let mut zip_handles = Vec::with_capacity(block_sizes.len());

    for &size in block_sizes {
        let compressed: Vec<u8> = bytes[i..i + size].to_vec();
        let handle = std::thread::spawn(move || gzip_decompress(&compressed));
        i += size;
        zip_handles.push(handle);
    }

    for handle in zip_handles {
        match handle.join() {
            Ok(decompressed) => result.push(decompressed?),
            Err(_) => {
                return Err(CodecError::UnknownError(
                    "Error joining decompression thread results".to_string(),
                ));
            }
        }
    }
    Ok(result.iter().flatten().copied().collect())
}

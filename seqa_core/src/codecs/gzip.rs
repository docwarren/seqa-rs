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

use std::io::prelude::*;
use std::io;
use flate2::read::GzDecoder;

// Uncompresses a Gz Encoded vector of bytes and returns a u8 vec or error
// Here &[u8] implements Read
pub fn gzip_decompress(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut gz = GzDecoder::new(bytes);
    let result = gz.read_to_end(&mut decompressed);
    match result {
        Ok(_) => Ok(decompressed),
        Err(e) => Err(e),
    }
}
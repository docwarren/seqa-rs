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

use crate::models::bam_header::header::BamHeaderError;

pub async fn read_magic(bytes: &Vec<u8>) -> Result<(String, u32), BamHeaderError> {
    let magic = String::from_utf8_lossy(&bytes[0..4]).to_string();
    let l_text = u32::from_le_bytes(bytes[4..8].try_into()?);

    Ok((magic, l_text))
}
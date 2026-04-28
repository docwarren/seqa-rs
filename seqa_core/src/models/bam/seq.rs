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

use serde::{Deserialize, Serialize};

use super::read_utils::map_sequence_code;

#[derive(Debug, Serialize, Deserialize)]
pub struct Seq {
    pub bytes: Vec<u8>
}

impl Seq {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Seq { bytes }
    }

    pub fn to_string(&self) -> String {
        if self.bytes.is_empty() {
            return String::from("*");
        }

        let mut i = 0;
        let mut result = String::new();
        let read_last_base: bool = self.bytes.len() % 2 == 0;

        while i < self.bytes.len() {
            let byte = self.bytes[i];
            let left: u8 = byte >> 4;
            let right: u8 = byte & 0x0F;
            let left_char = map_sequence_code(left);
            let right_char = map_sequence_code(right);
            result.push(left_char);
            if i < self.bytes.len() - 1 || read_last_base {
                result.push(right_char);
            }
            i += 1;
        }
        result
    }
}

impl Display for Seq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

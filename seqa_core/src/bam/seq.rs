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
use crate::api::parsing_error::ParsingError;
use crate::bam::read_utils::map_sequence_code;
use crate::traits::from_bytes::FromBytes;

#[derive(Debug, Serialize, Deserialize)]
pub struct Seq {
    pub bytes: Vec<u8>
}

impl Seq {
    pub fn new(bytes: &[u8]) -> Seq {
        Seq {
            bytes: bytes.to_vec()
        }
    }
    pub fn as_string(&self) -> String {
        if self.bytes.is_empty() {
            return String::from("*");
        }

        let mut i = 0;
        let mut result = String::new();
        let read_last_base: bool = self.bytes.len().is_multiple_of(2);

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

impl FromBytes for Seq {
    type Error = ParsingError;
    fn from_bytes(bytes: &[u8], i: usize) -> Result<(Self, usize), ParsingError> {
        Ok((Seq::new(bytes), bytes.len()))
    }
}

impl Display for Seq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

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
use crate::traits::from_bytes::FromBytes;

#[derive(Debug, Serialize, Deserialize)]
pub struct Qual {
    pub bytes: Vec<u8>,
}

impl Qual {

    pub fn as_string(&self) -> String {
        if self.bytes.is_empty() {
            String::from("*")
        }
        else if self.bytes.is_empty() && self.bytes[0] == 0xFF {
            String::from_utf8_lossy(&self.bytes).to_string()
        } else {
            self
                .bytes
                .iter()
                .map(|&q| (q + 33) as char) // Convert to ASCII quality scores
                .collect::<String>()
        }
    }
}

impl FromBytes for Qual {
    type Error = ParsingError;
    fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), ParsingError> {
        Ok((Qual {
            bytes: bytes.to_vec()
        }, bytes.len()))
    }
}

impl Display for Qual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}
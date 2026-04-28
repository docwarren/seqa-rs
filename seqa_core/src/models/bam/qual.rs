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

#[derive(Debug, Serialize, Deserialize)]
pub struct Qual {
    pub bytes: Vec<u8>,
}

impl Qual {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Qual { bytes }
    }

    pub fn to_string(&self) -> String {
        if self.bytes.is_empty() {
            return String::from("*");
        }
        if self.bytes.len() > 0 && self.bytes[0] == 0xFF {
            return String::from_utf8_lossy(&self.bytes).to_string();
        } else {
            let qual = self
                .bytes
                .iter()
                .map(|&q| (q + 33) as char) // Convert to ASCII quality scores
                .collect::<String>();
            return qual;
        }
    }
}

impl Display for Qual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
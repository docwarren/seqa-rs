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

use super::cigar_op::CigarOperation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Cigar {
    operations: Vec<CigarOperation>,
}

impl Cigar {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let mut operations = Vec::new();
        let mut i = 0;

        while i < bytes.len() {
            let op = u32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| "Invalid Cigar bytes".to_string())?);
            let cigar_op = CigarOperation::new(op);
            operations.push(cigar_op);
            i += 4;
        }

        Ok(Cigar { operations })
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn get_reference_length(&self) -> u32 {
        self.operations.iter().map(|op| op.get_reference_length()).sum()
    }

    pub fn get_read_length(&self) -> u32 {
        self.operations.iter().map(|op| op.get_read_length()).sum()
    }

    pub fn get_operations(&self) -> &Vec<CigarOperation> {
        &self.operations
    }
}

impl Display for Cigar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for op in self.operations.iter() {
            write!(f, "{}", op)?;
        }
        Ok(())
    }
}

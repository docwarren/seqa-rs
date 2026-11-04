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

use crate::api::parsing_error::ParsingError;
use crate::bam::cigar_op::CigarOperation;
use crate::traits::from_bytes::FromBytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Cigar {
    operations: Vec<CigarOperation>,
}

impl Cigar {
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn get_reference_length(&self) -> u32 {
        self.operations
            .iter()
            .map(|op| op.get_reference_length())
            .sum()
    }

    pub fn get_operations(&self) -> &[CigarOperation] {
        &self.operations
    }
}

impl FromBytes for Cigar {
    type Error = ParsingError;

    fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), Self::Error> {
        let mut operations = Vec::new();
        let mut idx = 0;
        while idx < bytes.len() {
            let op = u32::from_le_bytes(bytes[idx..idx + 4].try_into()?);
            let cigar_op = CigarOperation::new(op);
            operations.push(cigar_op);
            idx += 4;
        }

        Ok((Cigar { operations }, idx))
    }
}

impl Display for Cigar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "*")?;
        } else {
            for op in self.operations.iter() {
                write!(f, "{}", op)?;
            }
        };

        Ok(())
    }
}

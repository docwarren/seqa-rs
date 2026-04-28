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
pub struct CigarOperation {
    pub cigar_op: u32
}

impl CigarOperation {
    pub fn new(cigar_op: u32) -> Self {
        CigarOperation { cigar_op }
    }

    pub fn get_op(&self) -> u32 {
        self.cigar_op & 0xF
    }

    pub fn get_op_char(&self) -> char {
        match self.get_op() {
            0 => 'M',
            1 => 'I',
            2 => 'D',
            3 => 'N',
            4 => 'S',
            5 => 'H',
            6 => '=',
            7 => 'X',
            _ => '?',
        }
    }

    pub fn get_length(&self) -> u32 {
        self.cigar_op >> 4
    }

    pub fn get_reference_length(&self) -> u32 {
        match self.get_op() {
            0 | 2 | 3 | 7 | 8 => self.get_length(),
            _ => 0,
        }
    }

    pub fn get_read_length(&self) -> u32 {
        match self.get_op() {
            0 | 1 | 4 | 6 | 7 | 8 => self.get_length(),
            _ => 0,
        }
    }
}

impl Display for CigarOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.get_length(), self.get_op_char())
    }
}

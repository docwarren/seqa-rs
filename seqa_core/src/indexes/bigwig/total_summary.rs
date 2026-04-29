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

use serde::{Deserialize, Serialize};
use thiserror::Error;
use core::array::TryFromSliceError;

#[derive(Debug, Error)]
pub enum TotalSummaryError {
    #[error("Failed to parse TotalSummary: {0}")]
    ParseError(#[from] TryFromSliceError),

    #[error("Invalid TotalSummary data: {0}")]
    InvalidData(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalSummary {
    pub bases_covered: u64,
    pub min_val: f64,
    pub max_val: f64,
    pub sum_data: f64,
    pub sum_squares: f64,
}

impl TotalSummary {
    pub fn new() -> Self {
        TotalSummary {
            bases_covered: 0,
            min_val: f64::MAX,
            max_val: f64::MIN,
            sum_data: 0.0,
            sum_squares: 0.0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TotalSummaryError> {
        if bytes.len() < 40 {
            return Err(TotalSummaryError::InvalidData("Not enough bytes for a complete total summary".to_string()));
        }

        let bases_covered = u64::from_le_bytes(bytes[0..8].try_into()?);
        let min_val = f64::from_le_bytes(bytes[8..16].try_into()?);
        let max_val = f64::from_le_bytes(bytes[16..24].try_into()?);
        let sum_data = f64::from_le_bytes(bytes[24..32].try_into()?);
        let sum_squares = f64::from_le_bytes(bytes[32..40].try_into()?);

        Ok(TotalSummary {
            bases_covered,
            min_val,
            max_val,
            sum_data,
            sum_squares,
        })
    }
}
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

use crate::models::bam_header::header::BamHeaderError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderLine {
    pub code: String,
    pub tags: Vec<(String, String)>,
}

impl HeaderLine {
    pub fn from_line(line: String) -> Result<HeaderLine, BamHeaderError> {
        let tokens = line.split('\t').collect::<Vec<&str>>();
        if tokens.is_empty() {
            return Err(BamHeaderError::InvalidHeaderLine(line));
        }
        let code = tokens[0].to_string();
        let mut tags = Vec::new();
        for token in tokens.iter().skip(1) {
            let (key, value) = token.split_once(':').ok_or_else(|| BamHeaderError::InvalidHeaderLine(line.clone()))?;
            tags.push((key.to_string(), value.to_string()));
        }
        Ok(HeaderLine { code, tags })
    }
}

impl Display for HeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut tag_string = String::new();
        for (key, value) in &self.tags {
            tag_string.push_str(&format!("{}:{}", key, value));
            tag_string.push('\t');
        }
        // Remove the last tab character
        if !tag_string.is_empty() {
            tag_string.pop();
        }
        write!(f, "{}\t{}", self.code, tag_string)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_should_init_from_string() {
        let test_line1 = "@SQ\tSN:chr21\tLN:48129895".to_string();
        let test_line2 = "@PG\tID:bwa\tPN:bwa\tVN:0.6.1-r104-tpx".to_string();

        let expected_1 = test_line1.clone();
        let expected_2 = test_line2.clone();

        let line1 = HeaderLine::from_line(test_line1).expect("Failed to parse header line");
        assert_eq!(format!("{}", line1), expected_1);

        let line2 = HeaderLine::from_line(test_line2).expect("Failed to parse header line");
        assert_eq!(format!("{}", line2), expected_2);
    }
}
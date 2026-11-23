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

//! The `POST /search` response body.

use seqa_core::api::search_options::SearchOptions;
use seqa_core::api::search_result::SearchResult;
use seqa_core::ToLines;
use serde::{Deserialize, Serialize};

/// Body of a successful `POST /search` response, shared by every backend.
///
/// `lines[i]` holds the results for `coordinates[i]`. `header` is empty unless
/// the search asked for it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// The regions searched, in request order.
    pub coordinates: Vec<String>,
    /// The file header lines, or empty when the header was not requested.
    pub header: Vec<String>,
    /// One list of result lines per region. A list is empty when its region
    /// matched nothing or was invalid for the file.
    pub lines: Vec<Vec<String>>,
}

impl SearchResponse {
    /// Builds the response for `result`, filling `header` only when `options`
    /// set `include_header` or `header_only`.
    pub fn from_result(result: SearchResult, options: &SearchOptions) -> SearchResponse {
        let header = requested_header_lines(result.header, options);
        SearchResponse {
            coordinates: result.coordinates,
            header,
            lines: result.lines,
        }
    }
}

fn requested_header_lines(header: Option<Box<dyn ToLines>>, options: &SearchOptions) -> Vec<String> {
    let header_requested = options.include_header || options.header_only;
    match header {
        Some(header) if header_requested => header.to_lines(),
        _ => Vec::new(),
    }
}

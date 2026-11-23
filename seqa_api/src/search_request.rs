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

//! The `POST /search` request body and its validation.

use seqa_core::api::output_format::OutputFormat;
use seqa_core::api::parsing_error::ParsingError;
use seqa_core::api::search_options::SearchOptions;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Body of `POST /search`, shared by every backend.
///
/// `index_path` replaces the index inferred from `path`, and `output_format`
/// replaces the format inferred from its extension. Unknown fields are ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// URI of the data file.
    pub path: String,
    /// One or more one-based, inclusive regions, e.g. `chr1:1000-2000`.
    pub coordinates: Vec<String>,
    /// Return the file header with the results. Defaults to `false`.
    pub with_header: Option<bool>,
    /// Return the file header and no records. Defaults to `false`.
    pub only_header: Option<bool>,
    /// URI of the index file, when it is not next to the data file.
    pub index_path: Option<String>,
    /// File format name (case-insensitive), when it cannot be inferred from
    /// the extension of `path`.
    pub output_format: Option<String>,
}

/// Reasons a [`SearchRequest`] cannot be turned into [`SearchOptions`].
#[derive(Debug, Error)]
pub enum SearchRequestError {
    /// `coordinates` is empty.
    #[error("No coordinates")]
    NoCoordinates,

    /// A coordinate string does not parse.
    #[error("Invalid coordinates: {0}")]
    InvalidCoordinates(#[source] ParsingError),

    /// `output_format` does not name a known format. Holds the given name.
    #[error("Unknown output format: {0}")]
    UnknownOutputFormat(String),

    /// The format could not be determined from `output_format` or the
    /// extension of `path`. Holds the path.
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
}

impl SearchRequest {
    /// Validates the request and builds the [`SearchOptions`] for it.
    ///
    /// # Errors
    ///
    /// Returns [`SearchRequestError`] when the coordinates are empty or do not
    /// parse, when `output_format` names an unknown format, or when the format
    /// cannot be determined from `output_format` or the file extension.
    pub fn to_search_options(&self) -> Result<SearchOptions, SearchRequestError> {
        self.check_coordinates()?;
        let mut options = SearchOptions::new(&self.path, &self.coordinates)
            .map_err(SearchRequestError::InvalidCoordinates)?
            .set_include_header(self.with_header.unwrap_or(false))
            .set_header_only(self.only_header.unwrap_or(false));

        if let Some(index_path) = &self.index_path {
            options = options.set_index_path(index_path);
        }
        if let Some(output_format) = &self.output_format {
            options = options.set_output_format(output_format);
            check_output_format(&options, output_format)?;
        }
        self.check_format_resolved(&options)?;
        Ok(options)
    }

    fn check_coordinates(&self) -> Result<(), SearchRequestError> {
        if self.coordinates.is_empty() {
            return Err(SearchRequestError::NoCoordinates);
        }
        Ok(())
    }

    fn check_format_resolved(&self, options: &SearchOptions) -> Result<(), SearchRequestError> {
        if options.output_format == OutputFormat::STRING {
            return Err(SearchRequestError::UnsupportedFileType(self.path.clone()));
        }
        Ok(())
    }
}

fn check_output_format(options: &SearchOptions, output_format: &str) -> Result<(), SearchRequestError> {
    let resolved_name = options.output_format.to_string().to_lowercase();
    if resolved_name == output_format.to_lowercase() {
        return Ok(());
    }
    Err(SearchRequestError::UnknownOutputFormat(output_format.to_string()))
}

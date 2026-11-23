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

//! Errors from the notes module.

use seqa_core::stores::error::StoreError;
use thiserror::Error;

/// Errors from reading or saving a note.
#[derive(Debug, Error)]
pub enum NotesError {
    /// The path breaks a rule of
    /// [`validate_note_path`](crate::notes::validate_note_path). Holds the path.
    #[error("Invalid note path: {0}")]
    InvalidPath(String),

    /// The data to save is not valid standard base64.
    #[error("Invalid note data for {path}: {reason}")]
    InvalidData {
        /// The note path.
        path: String,
        /// Why the base64 decoder rejected the data.
        reason: String,
    },

    /// There is no object at the path. Holds the path.
    #[error("Note not found: {0}")]
    NotFound(String),

    /// The store failed for a reason other than a missing object.
    #[error("Failed to access note {path}: {source}")]
    Store {
        /// The note path.
        path: String,
        /// The underlying store error.
        #[source]
        source: StoreError,
    },
}

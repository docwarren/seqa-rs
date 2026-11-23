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

//! Reading and saving note files kept in a `notes` directory next to analysis data.

pub mod notes_error;

use seqa_core::stores::StoreService;
use seqa_core::stores::error::StoreError;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use notes_error::NotesError;
use serde::{Deserialize, Serialize};
use url::Url;

const NOTE_SCHEMES: [&str; 4] = ["file", "s3", "gs", "az"];
const NOTES_DIRECTORY: &str = "notes";
const AZURE_SCHEME: &str = "az";

/// Body of `POST /notes/read`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadNoteRequest {
    /// URI of the note; see [`validate_note_path`].
    pub path: String,
}

/// Body of a successful `POST /notes/read` response: the note as standard base64.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadNoteResponse {
    /// The note's bytes as standard base64 with padding.
    pub data: String,
}

/// Body of `POST /notes/save`: the note path and its contents as standard base64.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveNoteRequest {
    /// URI of the note; see [`validate_note_path`].
    pub path: String,
    /// The note's bytes as standard base64 with padding.
    pub data: String,
}

/// Checks that `path` is a `file`, `s3`, `gs` or `az` URI whose normalized object
/// path has a `notes` directory segment before a non-empty file name. For `az`
/// URIs the container (the first path segment) is not part of the object path.
///
/// # Errors
///
/// Returns [`NotesError::InvalidPath`] when any rule fails.
pub fn validate_note_path(path: &str) -> Result<(), NotesError> {
    let url = parse_note_url(path)?;
    check_note_scheme(&url, path)?;
    check_inside_notes_directory(&url, path)
}

/// Reads the note at `path` and returns its contents as standard base64.
///
/// # Errors
///
/// Returns [`NotesError::InvalidPath`] without contacting the store when the
/// path fails [`validate_note_path`], [`NotesError::NotFound`] when there is no
/// object at `path`, and [`NotesError::Store`] for any other store failure.
pub async fn read_note(store: &StoreService, path: &str) -> Result<ReadNoteResponse, NotesError> {
    validate_note_path(path)?;
    let bytes = store
        .get_object(path)
        .await
        .map_err(|error| to_notes_error(error, path))?;
    let data = STANDARD.encode(bytes);
    Ok(ReadNoteResponse { data })
}

/// Decodes the base64 in `request` and writes it to `request.path`, creating or
/// overwriting the object.
///
/// # Errors
///
/// Returns [`NotesError::InvalidPath`] or [`NotesError::InvalidData`] without
/// contacting the store when the path or data is invalid, and
/// [`NotesError::Store`] when the write fails.
pub async fn save_note(store: &StoreService, request: &SaveNoteRequest) -> Result<(), NotesError> {
    validate_note_path(&request.path)?;
    let bytes = decode_note_data(request)?;
    store
        .put_object(&request.path, &bytes)
        .await
        .map_err(|error| to_notes_error(error, &request.path))
}

fn decode_note_data(request: &SaveNoteRequest) -> Result<Vec<u8>, NotesError> {
    STANDARD
        .decode(&request.data)
        .map_err(|error| NotesError::InvalidData {
            path: request.path.clone(),
            reason: error.to_string(),
        })
}

fn to_notes_error(error: StoreError, path: &str) -> NotesError {
    match error {
        StoreError::ObjectStoreCreationError(object_store::Error::NotFound { .. }) => {
            NotesError::NotFound(path.to_string())
        }
        other => NotesError::Store {
            path: path.to_string(),
            source: other,
        },
    }
}

fn parse_note_url(path: &str) -> Result<Url, NotesError> {
    Url::parse(path).map_err(|_| invalid_path(path))
}

fn check_note_scheme(url: &Url, path: &str) -> Result<(), NotesError> {
    if NOTE_SCHEMES.contains(&url.scheme()) {
        return Ok(());
    }
    Err(invalid_path(path))
}

fn check_inside_notes_directory(url: &Url, path: &str) -> Result<(), NotesError> {
    let segments = object_path_segments(url);
    match segments.split_last() {
        Some((file_name, directories))
            if !file_name.is_empty() && directories.contains(&NOTES_DIRECTORY) =>
        {
            Ok(())
        }
        _ => Err(invalid_path(path)),
    }
}

fn object_path_segments(url: &Url) -> Vec<&str> {
    let mut segments: Vec<&str> = url.path_segments().map(Iterator::collect).unwrap_or_default();
    if url.scheme() == AZURE_SCHEME && !segments.is_empty() {
        segments.remove(0);
    }
    segments
}

fn invalid_path(path: &str) -> NotesError {
    NotesError::InvalidPath(path.to_string())
}

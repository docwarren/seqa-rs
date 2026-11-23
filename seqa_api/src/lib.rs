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

//! # seqa_api
//!
//! Request handling shared by the three seqa backends: `seqa_axum`,
//! `seqa_lambda` and the seqa Tauri app. Keeping it in one crate means every
//! backend accepts the same requests and returns the same responses, and
//! leaves `seqa_core` as a search engine with no API code.
//!
//! - [`search_request`] and [`search_response`]: the `POST /search` body, its
//!   conversion to [`SearchOptions`](seqa_core::api::search_options::SearchOptions),
//!   and the response built from a search result.
//! - [`list_directory_request`]: the `POST /files/list` body.
//! - [`notes`]: the note path rules and base64 read/save for `POST /notes/read`
//!   and `POST /notes/save`.
//! - [`local_uri`]: the guard the HTTP servers use to refuse URIs that would
//!   reach their own filesystem.

pub mod list_directory_request;
pub mod local_uri;
pub mod notes;
pub mod search_request;
pub mod search_response;

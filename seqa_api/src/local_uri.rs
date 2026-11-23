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

//! The guard that stops the HTTP servers touching their own filesystem.

use thiserror::Error;
use url::Url;

/// A local path was given where only remote URIs are allowed. Holds the
/// rejected URI.
#[derive(Error, Debug)]
#[error("Local paths are not available over HTTP: {0}")]
pub struct LocalUriError(pub String);

/// True for URIs that would resolve to the local filesystem: `file:` URIs
/// and anything that does not parse as a URL, which
/// [`StoreService`](seqa_core::stores::StoreService) treats as a local path.
///
/// The scheme comes from [`Url::parse`], the same parser the store uses, so
/// characters it strips (tabs, newlines, leading control characters) cannot
/// hide a `file:` scheme.
pub fn is_local_uri(uri: &str) -> bool {
    match Url::parse(uri) {
        Ok(url) => url.scheme() == "file",
        Err(_) => true,
    }
}

/// Refuses URIs that would resolve to the local filesystem.
///
/// Servers call this before listing, reading or writing on a client's behalf,
/// so that no client can reach the server's own filesystem.
///
/// # Errors
///
/// Returns [`LocalUriError`] when [`is_local_uri`] is true for `uri`.
pub fn reject_local_uri(uri: &str) -> Result<(), LocalUriError> {
    if is_local_uri(uri) {
        return Err(LocalUriError(uri.to_string()));
    }
    Ok(())
}

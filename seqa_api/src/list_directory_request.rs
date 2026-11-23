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

//! The `POST /files/list` request body.

use serde::{Deserialize, Serialize};

/// Body of `POST /files/list`.
///
/// A POST rather than a path parameter because URIs contain `://` and slashes,
/// which a path segment mangles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirectoryRequest {
    /// URI of the directory or bucket prefix whose direct entries are listed.
    pub uri: String,
}

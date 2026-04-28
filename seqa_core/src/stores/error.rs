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

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Failed to parse URL: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Failed to parse object store URI: {0}")]
    ObjectStoreUriParseError(String),

    #[error("Failed to create object store: {0}")]
    ObjectStoreCreationError(#[from] object_store::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Store not initialized")]
    StoreNotInitialized(String),

    #[error("List error: {0}")]
    ListError(String),

    #[error("Path error: {0}")]
    PathError(#[from] std::io::Error),

    #[error("Put error: {0}")]
    PutError(String)
}

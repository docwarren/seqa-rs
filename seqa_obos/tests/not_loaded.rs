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

//! The public API before any ontology is loaded. This binary never calls `load`.

use seqa_obos::{Error, SearchOptions, search};

#[test]
fn search_before_load_is_not_loaded() {
    let err = search("microceph", &SearchOptions::default()).unwrap_err();
    assert!(matches!(err, Error::NotLoaded), "{err:?}");
}

#[test]
fn short_query_before_load_is_not_loaded() {
    let err = search("hp", &SearchOptions::default()).unwrap_err();
    assert!(matches!(err, Error::NotLoaded), "{err:?}");
}

#[test]
fn zero_limit_is_checked_before_not_loaded() {
    let opts = SearchOptions {
        limit: 0,
        ..Default::default()
    };
    let err = search("microceph", &opts).unwrap_err();
    assert!(matches!(err, Error::InvalidArgument(_)), "{err:?}");
}

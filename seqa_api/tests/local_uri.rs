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

use seqa_api::list_directory_request::ListDirectoryRequest;
use seqa_api::local_uri::{is_local_uri, reject_local_uri};

#[test]
fn file_uri_is_local() {
    assert!(is_local_uri("file:///etc"));
}

#[test]
fn scheme_less_path_is_local() {
    assert!(is_local_uri("/etc/passwd"));
    assert!(is_local_uri("relative/notes/today.txt"));
}

#[test]
fn cloud_uris_are_not_local() {
    assert!(!is_local_uri("s3://bucket/analysis/"));
    assert!(!is_local_uri("gs://bucket/analysis/"));
    assert!(!is_local_uri("az://account/container/"));
}

#[test]
fn mixed_case_file_scheme_is_local() {
    assert!(is_local_uri("FiLe:///etc"));
}

#[test]
fn leading_whitespace_does_not_hide_file_scheme() {
    assert!(is_local_uri("  file:///etc"));
}

#[test]
fn list_directory_request_deserializes_uri() {
    let request: ListDirectoryRequest = serde_json::from_str(r#"{"uri": "s3://bucket/analysis/"}"#).unwrap();
    assert_eq!(request.uri, "s3://bucket/analysis/");
}

#[test]
fn list_directory_request_requires_uri() {
    let result = serde_json::from_str::<ListDirectoryRequest>(r#"{"wrong": "field"}"#);
    assert!(result.is_err());
}

#[test]
fn tab_inside_file_scheme_is_local() {
    assert!(is_local_uri("fi\tle:///etc"));
}

#[test]
fn newline_inside_file_scheme_is_local() {
    assert!(is_local_uri("file\n:///etc"));
}

#[test]
fn leading_control_character_does_not_hide_file_scheme() {
    assert!(is_local_uri("\u{1}file:///etc"));
}

#[test]
fn reject_local_uri_rejects_file_uri() {
    let error = reject_local_uri("file:///etc").unwrap_err();
    assert!(error.to_string().contains("file:///etc"), "{error}");
}

#[test]
fn reject_local_uri_allows_cloud_uri() {
    assert!(reject_local_uri("s3://bucket/analysis/").is_ok());
}

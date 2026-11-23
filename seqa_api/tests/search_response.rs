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

use seqa_core::ToLines;
use seqa_core::api::search_options::SearchOptions;
use seqa_api::search_response::SearchResponse;
use seqa_core::api::search_result::SearchResult;

struct FixedHeader;

impl ToLines for FixedHeader {
    fn to_lines(&self) -> Vec<String> {
        vec!["#CHROM\tPOS".to_string()]
    }
}

fn result_with_header() -> SearchResult {
    let mut result = SearchResult::new();
    result.header = Some(Box::new(FixedHeader));
    result.coordinates = vec!["chr1:1-10".to_string(), "chr2:1-10".to_string()];
    result.lines = vec![vec!["chr1\t5".to_string()], Vec::new()];
    result
}

fn options(include_header: bool, header_only: bool) -> SearchOptions {
    let coordinates = vec!["chr1:1-10".to_string(), "chr2:1-10".to_string()];
    SearchOptions::new("s3://bucket/file.vcf.gz", &coordinates)
        .unwrap()
        .set_include_header(include_header)
        .set_header_only(header_only)
}

#[test]
fn header_empty_when_not_requested() {
    let options = options(false, false);
    let response = SearchResponse::from_result(result_with_header(), &options);
    assert!(response.header.is_empty());
}

#[test]
fn header_filled_when_include_header() {
    let options = options(true, false);
    let response = SearchResponse::from_result(result_with_header(), &options);
    assert_eq!(response.header, vec!["#CHROM\tPOS".to_string()]);
}

#[test]
fn header_filled_when_header_only() {
    let options = options(false, true);
    let response = SearchResponse::from_result(result_with_header(), &options);
    assert_eq!(response.header, vec!["#CHROM\tPOS".to_string()]);
}

#[test]
fn header_empty_when_result_has_none() {
    let options = options(true, false);
    let mut result = result_with_header();
    result.header = None;
    let response = SearchResponse::from_result(result, &options);
    assert!(response.header.is_empty());
}

#[test]
fn one_lines_entry_per_coordinate() {
    let options = options(false, false);
    let response = SearchResponse::from_result(result_with_header(), &options);
    assert_eq!(response.coordinates, vec!["chr1:1-10".to_string(), "chr2:1-10".to_string()]);
    assert_eq!(response.lines.len(), 2);
    assert_eq!(response.lines[0], vec!["chr1\t5".to_string()]);
    assert!(response.lines[1].is_empty());
}

#[test]
fn serializes_field_names() {
    let options = options(true, false);
    let response = SearchResponse::from_result(result_with_header(), &options);
    let json = serde_json::to_value(&response).unwrap();
    assert!(json.get("coordinates").is_some());
    assert!(json.get("header").is_some());
    assert!(json.get("lines").is_some());
}

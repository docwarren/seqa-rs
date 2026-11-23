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

use seqa_core::api::output_format::OutputFormat;
use seqa_api::search_request::{SearchRequest, SearchRequestError};

fn parse_request(json: &str) -> SearchRequest {
    serde_json::from_str(json).unwrap()
}

#[test]
fn deserializes_snake_case_fields() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.bam", "coordinates": ["chr1:1-10"], "with_header": true, "only_header": false, "index_path": "s3://bucket/other.bai", "output_format": "bam"}"#,
    );
    assert_eq!(request.path, "s3://bucket/file.bam");
    assert_eq!(request.coordinates, vec!["chr1:1-10".to_string()]);
    assert_eq!(request.with_header, Some(true));
    assert_eq!(request.only_header, Some(false));
    assert_eq!(request.index_path.as_deref(), Some("s3://bucket/other.bai"));
    assert_eq!(request.output_format.as_deref(), Some("bam"));
}

#[test]
fn optional_fields_default_to_none() {
    let request = parse_request(r#"{"path": "s3://bucket/file.bam", "coordinates": ["chr1:1-10"]}"#);
    assert_eq!(request.with_header, None);
    assert_eq!(request.only_header, None);
    assert_eq!(request.index_path, None);
    assert_eq!(request.output_format, None);
}

#[test]
fn ignores_genome() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"], "genome": "hg19"}"#,
    );
    let options = request.to_search_options().unwrap();
    assert_eq!(options.genome, None);
}

#[test]
fn rejects_string_coordinates() {
    let result = serde_json::from_str::<SearchRequest>(
        r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": "chr1:1-1000"}"#,
    );
    assert!(result.is_err());
}

#[test]
fn rejects_empty_coordinates() {
    let request = parse_request(r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": []}"#);
    let error = request.to_search_options().unwrap_err();
    assert!(matches!(error, SearchRequestError::NoCoordinates));
    assert_eq!(error.to_string(), "No coordinates");
}

#[test]
fn rejects_unparseable_coordinate() {
    let request = parse_request(r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:z-q"]}"#);
    let error = request.to_search_options().unwrap_err();
    assert!(matches!(error, SearchRequestError::InvalidCoordinates(_)));
}

#[test]
fn rejects_unsupported_extension() {
    let request = parse_request(r#"{"path": "s3://bucket/file.xyz", "coordinates": ["chr1:1-10"]}"#);
    let error = request.to_search_options().unwrap_err();
    assert!(matches!(error, SearchRequestError::UnsupportedFileType(_)));
}

#[test]
fn rejects_unknown_output_format() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"], "output_format": "nonsense"}"#,
    );
    let error = request.to_search_options().unwrap_err();
    assert!(matches!(error, SearchRequestError::UnknownOutputFormat(_)));
}

#[test]
fn rejects_string_output_format() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.xyz", "coordinates": ["chr1:1-10"], "output_format": "string"}"#,
    );
    let error = request.to_search_options().unwrap_err();
    assert!(matches!(error, SearchRequestError::UnsupportedFileType(_)));
}

#[test]
fn infers_format_and_index_from_extension() {
    let request = parse_request(r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"]}"#);
    let options = request.to_search_options().unwrap();
    assert_eq!(options.output_format, OutputFormat::VCF);
    assert_eq!(options.index_path, "s3://bucket/file.vcf.gz.tbi");
}

#[test]
fn output_format_replaces_inferred_format() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.xyz", "coordinates": ["chr1:1-10"], "output_format": "bigwig"}"#,
    );
    let options = request.to_search_options().unwrap();
    assert_eq!(options.output_format, OutputFormat::BIGWIG);
}

#[test]
fn output_format_is_case_insensitive() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.xyz", "coordinates": ["chr1:1-10"], "output_format": "BAM"}"#,
    );
    let options = request.to_search_options().unwrap();
    assert_eq!(options.output_format, OutputFormat::BAM);
}

#[test]
fn index_path_replaces_inferred_index() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.bam", "coordinates": ["chr1:1-10"], "index_path": "s3://other/index.bai"}"#,
    );
    let options = request.to_search_options().unwrap();
    assert_eq!(options.index_path, "s3://other/index.bai");
}

#[test]
fn header_flags_default_to_false() {
    let request = parse_request(r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"]}"#);
    let options = request.to_search_options().unwrap();
    assert!(!options.include_header);
    assert!(!options.header_only);
}

#[test]
fn header_flags_are_applied() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"], "with_header": true, "only_header": true}"#,
    );
    let options = request.to_search_options().unwrap();
    assert!(options.include_header);
    assert!(options.header_only);
}

#[test]
fn keeps_coordinates_in_order() {
    let request = parse_request(
        r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr2:5-6", "chr1:1-10"]}"#,
    );
    let options = request.to_search_options().unwrap();
    assert_eq!(options.coordinates.len(), 2);
    assert_eq!(options.coordinates[0].get_chromosome(), "chr2");
    assert_eq!(options.coordinates[1].get_chromosome(), "chr1");
}

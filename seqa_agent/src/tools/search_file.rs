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

use std::sync::Arc;

use rig_agent::tool::{Tool, ToolContext as RigToolContext, ToolExecutionError};
use seqa_core::api::parsing_error::ParsingError;
use seqa_core::api::search::search_features;
use seqa_core::api::search_error::SearchError;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::api::search_result::SearchResult;
use seqa_core::coordinate::Coordinate;
use seqa_core::models::coordinate_system::CoordinateSystem;
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

use super::{LocalPathNotAllowed, ToolContext, model_visible_error};
use crate::summary::count_summary;

#[derive(Debug, Error)]
pub(crate) enum SearchFileError {
    #[error(transparent)]
    LocalPath(#[from] LocalPathNotAllowed),
    #[error("Invalid coordinate {coordinate}: {source}")]
    InvalidCoordinate {
        coordinate: String,
        source: ParsingError,
    },
    #[error("Could not build the search options for {path}: {source}")]
    Options { path: String, source: ParsingError },
    #[error("Search of {path} failed: {source}")]
    Search { path: String, source: SearchError },
}

#[derive(Deserialize)]
pub(crate) struct SearchFileArguments {
    path: String,
    coordinates: Vec<String>,
}

pub(crate) struct SearchFile {
    pub(super) context: Arc<ToolContext>,
}

impl Tool for SearchFile {
    const NAME: &'static str = "search_file";
    type Args = SearchFileArguments;
    type Output = Value;
    type Error = SearchFileError;

    fn description(&self) -> String {
        "Search one genomic file (BAM, VCF, BED, BedGraph, GFF, GTF, FASTA, BigWig, BigBed) for one or more regions and send the header and records to the user's display. You receive only the header line count and record count.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File URI: s3://, gs://, az://, http(s)://, file:// or a local path. The index is found next to it." },
                "coordinates": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "One-based regions such as chr1:1000-2000, or a whole chromosome such as chr1"
                }
            },
            "required": ["path", "coordinates"]
        })
    }

    fn map_error(&self, error: Self::Error) -> ToolExecutionError {
        model_visible_error(error)
    }

    async fn call(
        &self,
        _rig_context: &mut RigToolContext,
        arguments: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        self.context.check_local_path(&arguments.path)?;
        let options = build_options(&arguments)?;
        let result = search_features(&self.context.store_service, &options)
            .await
            .map_err(|source| SearchFileError::Search {
                path: arguments.path.clone(),
                source,
            })?;
        let header = header_items(&result);
        let records = record_items(&result);
        let header_line_count = header.len();
        let record_count = records.len();
        let items = [header, records].concat();
        let extra_counts = [("header_line_count", header_line_count)];
        Ok(count_summary(
            &self.context,
            Self::NAME,
            items,
            record_count,
            &extra_counts,
        ))
    }
}

fn build_options(arguments: &SearchFileArguments) -> Result<SearchOptions, SearchFileError> {
    check_coordinates(&arguments.coordinates)?;
    SearchOptions::new(&arguments.path, &arguments.coordinates).map_err(|source| {
        SearchFileError::Options {
            path: arguments.path.clone(),
            source,
        }
    })
}

fn check_coordinates(coordinates: &[String]) -> Result<(), SearchFileError> {
    for coordinate in coordinates {
        Coordinate::new(coordinate, CoordinateSystem::OneBasedClosed).map_err(|source| {
            SearchFileError::InvalidCoordinate {
                coordinate: coordinate.clone(),
                source,
            }
        })?;
    }
    Ok(())
}

fn header_items(result: &SearchResult) -> Vec<Value> {
    let lines = result
        .header
        .as_ref()
        .map(|header| header.to_lines())
        .unwrap_or_default();
    lines
        .into_iter()
        .map(|line| json!({ "kind": "header", "line": line }))
        .collect()
}

fn record_items(result: &SearchResult) -> Vec<Value> {
    result
        .coordinates
        .iter()
        .zip(&result.lines)
        .flat_map(|(coordinate, lines)| coordinate_record_items(coordinate, lines))
        .collect()
}

fn coordinate_record_items(coordinate: &str, lines: &[String]) -> Vec<Value> {
    lines
        .iter()
        .map(|line| json!({ "kind": "record", "coordinate": coordinate, "line": line }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::{call_tool, summary_keys, test_context};

    const VCF_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../seqa_core/mock_data/NA12878.gatk.cnv.vcf.gz"
    );
    const TEXT_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../seqa_core/mock_data/test.txt"
    );

    fn search_arguments(path: &str, coordinates: &[&str]) -> SearchFileArguments {
        SearchFileArguments {
            path: path.to_string(),
            coordinates: coordinates
                .iter()
                .map(|coordinate| coordinate.to_string())
                .collect(),
        }
    }

    fn items_of_kind<'a>(items: &'a [Value], kind: &str) -> Vec<&'a Value> {
        items.iter().filter(|item| item["kind"] == kind).collect()
    }

    #[tokio::test]
    async fn full_result_holds_header_lines_then_records() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = SearchFile { context };
        let arguments = search_arguments(VCF_PATH, &["chr1:30000-50000"]);
        call_tool(&tool, arguments).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        let first = &event.items[0];
        assert_eq!(first["kind"], "header");
        assert!(first["line"].as_str().unwrap().starts_with("##fileformat="));
        let first_record = event
            .items
            .iter()
            .position(|item| item["kind"] == "record")
            .unwrap();
        let headers_after_records = items_of_kind(&event.items[first_record..], "header");
        assert!(headers_after_records.is_empty());
        let records = items_of_kind(&event.items, "record");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["coordinate"], "chr1:30000-50000");
        assert!(
            records[0]["line"]
                .as_str()
                .unwrap()
                .contains("NA12878_DUP_chr1_1")
        );
    }

    #[tokio::test]
    async fn model_receives_only_counts() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = SearchFile { context };
        let arguments = search_arguments(VCF_PATH, &["chr1:30000-50000", "chr1:110000-130000"]);
        let summary = call_tool(&tool, arguments).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        let header_count = items_of_kind(&event.items, "header").len();
        let record_count = items_of_kind(&event.items, "record").len();
        assert_eq!(
            summary_keys(&summary),
            ["header_line_count", "record_count", "result_id", "tool"]
        );
        assert_eq!(summary["header_line_count"], header_count);
        assert_eq!(summary["record_count"], record_count);
        assert_eq!(record_count, 3);
        assert_eq!(summary["result_id"], event.result_id);
    }

    #[tokio::test]
    async fn bad_coordinate_error_names_it() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = SearchFile { context };
        let arguments = search_arguments(VCF_PATH, &["chr1:1-100", "chr1:abc"]);
        let error = call_tool(&tool, arguments).await.unwrap_err();
        assert!(error.to_string().contains("chr1:abc"), "{error}");
        assert!(!error.to_string().contains("chr1:1-100"), "{error}");
        assert!(event_receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn unsupported_extension_gives_an_unsupported_format_error() {
        let (context, _event_receiver) = test_context(20, true);
        let tool = SearchFile { context };
        let arguments = search_arguments(TEXT_PATH, &["chr1:1-100"]);
        let error = call_tool(&tool, arguments).await.unwrap_err();
        assert!(
            error.to_string().contains("Unsupported file format"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn local_path_is_rejected_when_disallowed() {
        let (context, mut event_receiver) = test_context(20, false);
        let tool = SearchFile { context };
        let arguments = search_arguments(VCF_PATH, &["chr1:30000-50000"]);
        let error = call_tool(&tool, arguments).await.unwrap_err();
        assert!(
            error.to_string().starts_with("Local paths are not allowed"),
            "{error}"
        );
        assert!(event_receiver.try_recv().is_err());
    }
}

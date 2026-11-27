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

mod cytobands;
mod gene_coordinates;
mod list_buckets;
mod list_directory;
mod search_file;
mod search_ontology;
mod supported_formats;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rig_agent::AgentBuilder;
use rig_agent::agent::WithBuilderTools;
use rig_agent::tool::ToolExecutionError;
use seqa_api::local_uri::is_local_uri;
use seqa_core::stores::StoreService;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use url::Url;

use crate::events::ToolResultEvent;
use cytobands::Cytobands;
use gene_coordinates::GeneCoordinates;
use list_buckets::ListBuckets;
use list_directory::ListDirectory;
use search_file::SearchFile;
use search_ontology::SearchOntology;
use supported_formats::SupportedFormats;

#[derive(Debug, Error)]
#[error("Local paths are not allowed: {0}")]
pub(crate) struct LocalPathNotAllowed(String);

pub(crate) struct ToolContext {
    pub(crate) store_service: StoreService,
    pub(crate) summary_limit: usize,
    pub(crate) allow_local_paths: bool,
    pub(crate) event_sender: UnboundedSender<ToolResultEvent>,
    result_counter: AtomicU64,
}

impl ToolContext {
    pub(crate) fn new(
        summary_limit: usize,
        allow_local_paths: bool,
        event_sender: UnboundedSender<ToolResultEvent>,
    ) -> Self {
        ToolContext {
            store_service: StoreService::new(),
            summary_limit,
            allow_local_paths,
            event_sender,
            result_counter: AtomicU64::new(0),
        }
    }

    pub(crate) fn next_result_id(&self) -> String {
        let previous = self.result_counter.fetch_add(1, Ordering::Relaxed);
        format!("r{}", previous + 1)
    }

    pub(crate) fn check_local_path(&self, uri: &str) -> Result<(), LocalPathNotAllowed> {
        let is_local = is_local_uri(uri) || is_windows_drive_path(uri);
        if !self.allow_local_paths && is_local {
            return Err(LocalPathNotAllowed(uri.to_string()));
        }
        Ok(())
    }
}

pub(crate) fn add_built_in_tools(
    builder: AgentBuilder,
    context: &Arc<ToolContext>,
) -> AgentBuilder<WithBuilderTools> {
    let search_file = SearchFile {
        context: Arc::clone(context),
    };
    let list_directory = ListDirectory {
        context: Arc::clone(context),
    };
    let list_buckets = ListBuckets {
        context: Arc::clone(context),
    };
    let supported_formats = SupportedFormats {
        context: Arc::clone(context),
    };
    let gene_coordinates = GeneCoordinates {
        context: Arc::clone(context),
    };
    let cytobands = Cytobands {
        context: Arc::clone(context),
    };
    let search_ontology = SearchOntology {
        context: Arc::clone(context),
    };
    builder
        .tool(search_file)
        .tool(list_directory)
        .tool(list_buckets)
        .tool(supported_formats)
        .tool(gene_coordinates)
        .tool(cytobands)
        .tool(search_ontology)
}

pub(crate) fn is_windows_drive_path(uri: &str) -> bool {
    match Url::parse(uri) {
        Ok(url) => url.scheme().len() == 1,
        Err(_) => false,
    }
}

fn model_visible_error<E>(error: E) -> ToolExecutionError
where
    E: std::error::Error + Send + Sync + 'static,
{
    let message = error.to_string();
    ToolExecutionError::other(message).with_source(error)
}

fn serialize_items<T: Serialize>(values: &[T]) -> Result<Vec<Value>, serde_json::Error> {
    values.iter().map(serde_json::to_value).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use rig_agent::tool::{Tool, ToolContext as RigToolContext};
    use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

    pub(crate) async fn call_tool<T: Tool>(
        tool: &T,
        arguments: T::Args,
    ) -> Result<T::Output, T::Error> {
        let mut rig_context = RigToolContext::new();
        tool.call(&mut rig_context, arguments).await
    }

    pub(crate) fn summary_keys(summary: &Value) -> Vec<&str> {
        summary
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect()
    }

    pub(crate) fn test_context(
        summary_limit: usize,
        allow_local_paths: bool,
    ) -> (Arc<ToolContext>, UnboundedReceiver<ToolResultEvent>) {
        let (event_sender, event_receiver) = unbounded_channel();
        let context = ToolContext::new(summary_limit, allow_local_paths, event_sender);
        let shared_context = Arc::new(context);
        (shared_context, event_receiver)
    }

    #[test]
    fn result_ids_are_unique_and_increase() {
        let (context, _event_receiver) = test_context(20, true);
        let first = context.next_result_id();
        let second = context.next_result_id();
        let third = context.next_result_id();
        assert_eq!(first, "r1");
        assert_eq!(second, "r2");
        assert_eq!(third, "r3");
    }

    #[test]
    fn local_paths_are_rejected_when_disallowed() {
        let (context, _event_receiver) = test_context(20, false);
        let plain_path = context.check_local_path("/data/x").unwrap_err();
        let file_uri = context.check_local_path("file:///x").unwrap_err();
        assert_eq!(
            plain_path.to_string(),
            "Local paths are not allowed: /data/x"
        );
        assert_eq!(
            file_uri.to_string(),
            "Local paths are not allowed: file:///x"
        );
    }

    #[test]
    fn windows_drive_paths_are_recognised() {
        assert!(is_windows_drive_path("C:\\data"));
        assert!(is_windows_drive_path("d:/data/x.vcf.gz"));
        assert!(!is_windows_drive_path("s3://b/x"));
        assert!(!is_windows_drive_path("/data/x"));
    }

    #[test]
    fn windows_drive_paths_are_rejected_when_disallowed() {
        let (context, _event_receiver) = test_context(20, false);
        let error = context.check_local_path("C:\\data").unwrap_err();
        assert_eq!(error.to_string(), "Local paths are not allowed: C:\\data");
    }

    #[test]
    fn cloud_paths_are_always_allowed() {
        let (disallowing_context, _first_receiver) = test_context(20, false);
        let (allowing_context, _second_receiver) = test_context(20, true);
        assert!(disallowing_context.check_local_path("s3://b/x").is_ok());
        assert!(allowing_context.check_local_path("s3://b/x").is_ok());
    }

    #[test]
    fn tool_errors_are_visible_to_the_model() {
        let error = LocalPathNotAllowed("/data/x".to_string());
        let execution_error = model_visible_error(error);
        assert_eq!(
            execution_error.model_feedback(),
            Some("Local paths are not allowed: /data/x")
        );
    }

    #[test]
    fn local_paths_are_allowed_when_allowed() {
        let (context, _event_receiver) = test_context(20, true);
        assert!(context.check_local_path("/data/x").is_ok());
        assert!(context.check_local_path("file:///x").is_ok());
    }
}

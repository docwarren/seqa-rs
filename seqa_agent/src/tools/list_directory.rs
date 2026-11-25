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
use seqa_core::stores::error::StoreError;
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;
use url::Url;

use super::{
    LocalPathNotAllowed, ToolContext, is_windows_drive_path, model_visible_error, serialize_items,
};
use crate::summary::item_summary;

#[derive(Debug, Error)]
pub(crate) enum ListDirectoryError {
    #[error(transparent)]
    LocalPath(#[from] LocalPathNotAllowed),
    #[error("Could not list {uri}: {source}")]
    List { uri: String, source: StoreError },
    #[error("Could not serialize the listing of {uri}: {source}")]
    Serialize {
        uri: String,
        source: serde_json::Error,
    },
}

#[derive(Deserialize)]
pub(crate) struct ListDirectoryArguments {
    uri: String,
}

pub(crate) struct ListDirectory {
    pub(super) context: Arc<ToolContext>,
}

impl Tool for ListDirectory {
    const NAME: &'static str = "list_directory";
    type Args = ListDirectoryArguments;
    type Output = Value;
    type Error = ListDirectoryError;

    fn description(&self) -> String {
        "List the folders and files directly under a storage URI, with each entry's URI, name, whether it is a directory, and size.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "uri": { "type": "string", "description": "Folder URI: s3://bucket/prefix/, gs://, az://, file:// or a local path" }
            },
            "required": ["uri"]
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
        self.context.check_local_path(&arguments.uri)?;
        let store_uri = to_store_uri(&arguments.uri);
        let entries = self
            .context
            .store_service
            .list_directory(&store_uri)
            .await
            .map_err(|source| ListDirectoryError::List {
                uri: arguments.uri.clone(),
                source,
            })?;
        let items = serialize_items(&entries).map_err(|source| ListDirectoryError::Serialize {
            uri: arguments.uri.clone(),
            source,
        })?;
        Ok(item_summary(&self.context, Self::NAME, items))
    }
}

fn to_store_uri(uri: &str) -> String {
    if is_store_url(uri) {
        return uri.to_string();
    }
    local_path_uri(uri).unwrap_or_else(|| uri.to_string())
}

fn is_store_url(uri: &str) -> bool {
    Url::parse(uri).is_ok() && !is_windows_drive_path(uri)
}

fn local_path_uri(path: &str) -> Option<String> {
    let absolute_path = std::path::absolute(path).ok()?;
    let url = Url::from_file_path(absolute_path).ok()?;
    Some(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::{call_tool, test_context};

    fn mock_data_directory() -> String {
        let relative = concat!(env!("CARGO_MANIFEST_DIR"), "/../seqa_core/mock_data/");
        let canonical = std::fs::canonicalize(relative).unwrap();
        format!("{}/", canonical.display())
    }

    fn list_arguments(uri: &str) -> ListDirectoryArguments {
        ListDirectoryArguments {
            uri: uri.to_string(),
        }
    }

    fn has_vcf(items: &[Value]) -> bool {
        items
            .iter()
            .any(|entry| entry["name"] == "NA12878.gatk.cnv.vcf.gz")
    }

    #[tokio::test]
    async fn plain_local_path_lists_the_vcf() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = ListDirectory { context };
        let directory = mock_data_directory();
        let summary = call_tool(&tool, list_arguments(&directory)).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        assert!(has_vcf(&event.items));
        assert_eq!(summary["total"], event.items.len());
    }

    #[tokio::test]
    async fn path_with_parent_segments_lists_the_vcf() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = ListDirectory { context };
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../seqa_core/mock_data/");
        call_tool(&tool, list_arguments(directory)).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        assert!(has_vcf(&event.items));
    }

    #[test]
    fn windows_drive_path_becomes_a_file_uri() {
        let store_uri = to_store_uri("C:\\data");
        assert!(store_uri.starts_with("file://"), "{store_uri}");
    }

    #[tokio::test]
    async fn file_uri_lists_the_vcf() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = ListDirectory { context };
        let directory = format!("file://{}", mock_data_directory());
        call_tool(&tool, list_arguments(&directory)).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        assert!(has_vcf(&event.items));
    }

    #[tokio::test]
    async fn store_error_includes_the_uri() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = ListDirectory { context };
        let error = call_tool(&tool, list_arguments("ftp://example.com/data/"))
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("ftp://example.com/data/"),
            "{error}"
        );
        assert!(event_receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn local_path_is_rejected_when_disallowed() {
        let (context, mut event_receiver) = test_context(20, false);
        let tool = ListDirectory { context };
        let directory = mock_data_directory();
        let error = call_tool(&tool, list_arguments(&directory))
            .await
            .unwrap_err();
        assert!(
            error.to_string().starts_with("Local paths are not allowed"),
            "{error}"
        );
        assert!(event_receiver.try_recv().is_err());
    }
}

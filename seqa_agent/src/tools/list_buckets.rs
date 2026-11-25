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
use seqa_core::stores::buckets::{CloudBackend, list_buckets};
use seqa_core::stores::error::StoreError;
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

use super::{ToolContext, model_visible_error, serialize_items};
use crate::summary::item_summary;

#[derive(Debug, Error)]
pub(crate) enum ListBucketsError {
    #[error("Could not list buckets for backend {backend}: {source}")]
    List { backend: String, source: StoreError },
    #[error("Could not serialize the buckets for backend {backend}: {source}")]
    Serialize {
        backend: String,
        source: serde_json::Error,
    },
}

#[derive(Deserialize)]
pub(crate) struct ListBucketsArguments {
    backend: String,
}

pub(crate) struct ListBuckets {
    context: Arc<ToolContext>,
}

impl ListBuckets {
    pub(crate) fn new(context: &Arc<ToolContext>) -> Self {
        ListBuckets {
            context: Arc::clone(context),
        }
    }
}

impl Tool for ListBuckets {
    const NAME: &'static str = "list_buckets";
    type Args = ListBucketsArguments;
    type Output = Value;
    type Error = ListBucketsError;

    fn description(&self) -> String {
        "List the buckets (Azure: containers) the configured credentials can see on one cloud backend, as URIs that list_directory accepts.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "backend": { "type": "string", "enum": ["s3", "gcs", "azure"], "description": "Cloud backend" }
            },
            "required": ["backend"]
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
        let buckets =
            fetch_buckets(&arguments.backend)
                .await
                .map_err(|source| ListBucketsError::List {
                    backend: arguments.backend.clone(),
                    source,
                })?;
        let items = serialize_items(&buckets).map_err(|source| ListBucketsError::Serialize {
            backend: arguments.backend.clone(),
            source,
        })?;
        Ok(item_summary(&self.context, Self::NAME, items))
    }
}

async fn fetch_buckets(backend: &str) -> Result<Vec<String>, StoreError> {
    let cloud_backend = CloudBackend::from_id(backend)?;
    list_buckets(cloud_backend).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::{call_tool, test_context};

    #[tokio::test]
    async fn unknown_backend_error_names_it() {
        let (context, mut event_receiver) = test_context(20, true);
        let shared_context = Arc::new(context);
        let tool = ListBuckets::new(&shared_context);
        let arguments = ListBucketsArguments {
            backend: "ftp".to_string(),
        };
        let error = call_tool(&tool, arguments).await.unwrap_err();
        assert!(error.to_string().contains("ftp"), "{error}");
        assert!(event_receiver.try_recv().is_err());
    }
}

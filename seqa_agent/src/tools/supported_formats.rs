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
use seqa_core::utils::supported_formats;
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

use super::{ToolContext, model_visible_error, serialize_items};
use crate::summary::item_summary;

#[derive(Debug, Error)]
pub(crate) enum SupportedFormatsError {
    #[error("Could not serialize the supported formats: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Deserialize)]
pub(crate) struct SupportedFormatsArguments {}

pub(crate) struct SupportedFormats {
    context: Arc<ToolContext>,
}

impl SupportedFormats {
    pub(crate) fn new(context: &Arc<ToolContext>) -> Self {
        SupportedFormats {
            context: Arc::clone(context),
        }
    }
}

impl Tool for SupportedFormats {
    const NAME: &'static str = "supported_formats";
    type Args = SupportedFormatsArguments;
    type Output = Value;
    type Error = SupportedFormatsError;

    fn description(&self) -> String {
        "List the genomic file formats seqa can search, with their file extensions and index extension.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    fn map_error(&self, error: Self::Error) -> ToolExecutionError {
        model_visible_error(error)
    }

    async fn call(
        &self,
        _rig_context: &mut RigToolContext,
        _arguments: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let formats = supported_formats();
        let items = serialize_items(&formats)?;
        Ok(item_summary(&self.context, Self::NAME, items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::{call_tool, test_context};

    fn find_format<'a>(items: &'a [Value], format: &str) -> &'a Value {
        items.iter().find(|item| item["format"] == format).unwrap()
    }

    #[tokio::test]
    async fn full_result_lists_bam_with_an_index_and_bigwig_without() {
        let (context, mut event_receiver) = test_context(20, true);
        let shared_context = Arc::new(context);
        let tool = SupportedFormats::new(&shared_context);
        let summary = call_tool(&tool, SupportedFormatsArguments {})
            .await
            .unwrap();
        let event = event_receiver.try_recv().unwrap();
        let bam = find_format(&event.items, "BAM");
        let bigwig = find_format(&event.items, "BIGWIG");
        assert_eq!(bam["indexExtension"], ".bai");
        assert_eq!(bigwig["indexExtension"], Value::Null);
        assert_eq!(summary["tool"], "supported_formats");
        assert_eq!(summary["omitted"], 0);
    }
}

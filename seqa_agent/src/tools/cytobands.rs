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
use seqa_core::reference_data::cytobands;
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

use super::{ToolContext, model_visible_error, serialize_items};
use crate::summary::record_count_summary;

#[derive(Debug, Error)]
pub(crate) enum CytobandsError {
    #[error("Could not serialize the cytobands: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Deserialize)]
pub(crate) struct CytobandsArguments {
    genome: String,
    chromosome: String,
}

pub(crate) struct Cytobands {
    context: Arc<ToolContext>,
}

impl Cytobands {
    pub(crate) fn new(context: &Arc<ToolContext>) -> Self {
        Cytobands {
            context: Arc::clone(context),
        }
    }
}

impl Tool for Cytobands {
    const NAME: &'static str = "cytobands";
    type Args = CytobandsArguments;
    type Output = Value;
    type Error = CytobandsError;

    fn description(&self) -> String {
        "Send the cytobands (chromosome, begin, end, name, stain) of one chromosome to the user's display. You receive only the record count.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "genome": { "type": "string", "description": "Genome build: hg38 or hg19" },
                "chromosome": { "type": "string", "description": "Chromosome name with the chr prefix, e.g. chr17" }
            },
            "required": ["genome", "chromosome"]
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
        let bands = cytobands(&arguments.genome, &arguments.chromosome);
        let items = serialize_items(&bands)?;
        Ok(record_count_summary(&self.context, Self::NAME, items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::{call_tool, summary_keys, test_context};

    fn hg38(chromosome: &str) -> CytobandsArguments {
        CytobandsArguments {
            genome: "hg38".to_string(),
            chromosome: chromosome.to_string(),
        }
    }

    #[tokio::test]
    async fn full_result_holds_the_chr17_bands() {
        let (context, mut event_receiver) = test_context(20, true);
        let shared_context = Arc::new(context);
        let tool = Cytobands::new(&shared_context);
        call_tool(&tool, hg38("chr17")).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        assert!(!event.items.is_empty());
        assert!(event.items.iter().all(|band| band["chromosome"] == "chr17"));
        assert!(event.items.iter().any(|band| band["name"] == "q21.31"));
    }

    #[tokio::test]
    async fn model_receives_only_the_record_count() {
        let (context, mut event_receiver) = test_context(20, true);
        let shared_context = Arc::new(context);
        let tool = Cytobands::new(&shared_context);
        let summary = call_tool(&tool, hg38("chr17")).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        assert_eq!(
            summary_keys(&summary),
            ["record_count", "result_id", "tool"]
        );
        assert_eq!(summary["record_count"], event.items.len());
    }

    #[tokio::test]
    async fn chromosome_without_prefix_gives_zero_records() {
        let (context, _event_receiver) = test_context(20, true);
        let shared_context = Arc::new(context);
        let tool = Cytobands::new(&shared_context);
        let summary = call_tool(&tool, hg38("17")).await.unwrap();
        assert_eq!(summary["record_count"], 0);
    }
}

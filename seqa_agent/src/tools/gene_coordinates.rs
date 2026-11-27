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
use seqa_core::reference_data::gene_coordinates;
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

use super::{ToolContext, model_visible_error, serialize_items};
use crate::summary::record_count_summary;

#[derive(Debug, Error)]
pub(crate) enum GeneCoordinatesError {
    #[error("Could not serialize the gene coordinates: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Deserialize)]
pub(crate) struct GeneCoordinatesArguments {
    genome: String,
    source: String,
}

pub(crate) struct GeneCoordinates {
    pub(super) context: Arc<ToolContext>,
}

impl Tool for GeneCoordinates {
    const NAME: &'static str = "gene_coordinates";
    type Args = GeneCoordinatesArguments;
    type Output = Value;
    type Error = GeneCoordinatesError;

    fn description(&self) -> String {
        "Send every gene for a genome build and source (chromosome, begin, end, symbol) to the user's display. You receive only the record count.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "genome": { "type": "string", "description": "Genome build: hg38 or hg19" },
                "source": { "type": "string", "description": "Gene source: refseq or gencode" }
            },
            "required": ["genome", "source"]
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
        let genes = gene_coordinates(&arguments.genome, &arguments.source);
        let items = serialize_items(genes)?;
        Ok(record_count_summary(&self.context, Self::NAME, items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::{call_tool, summary_keys, test_context};

    fn hg38_refseq() -> GeneCoordinatesArguments {
        GeneCoordinatesArguments {
            genome: "hg38".to_string(),
            source: "refseq".to_string(),
        }
    }

    #[tokio::test]
    async fn full_result_contains_brca1_on_chr17() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = GeneCoordinates { context };
        call_tool(&tool, hg38_refseq()).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        let brca1 = event
            .items
            .iter()
            .find(|gene| gene["symbol"] == "BRCA1")
            .unwrap();
        assert_eq!(brca1["chromosome"], "chr17");
    }

    #[tokio::test]
    async fn model_receives_only_the_record_count() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = GeneCoordinates { context };
        let summary = call_tool(&tool, hg38_refseq()).await.unwrap();
        let event = event_receiver.try_recv().unwrap();
        assert_eq!(
            summary_keys(&summary),
            ["record_count", "result_id", "tool"]
        );
        assert_eq!(summary["record_count"], event.items.len());
        assert_eq!(summary["result_id"], event.result_id);
    }
}

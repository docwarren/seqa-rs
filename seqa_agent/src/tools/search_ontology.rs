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
use seqa_obos::{SearchOptions, search};
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

use super::{ToolContext, model_visible_error, serialize_items};
use crate::summary::item_summary;

#[derive(Debug, Error)]
pub(crate) enum SearchOntologyError {
    #[error("Ontology search for {query} failed: {source}")]
    Search {
        query: String,
        source: seqa_obos::Error,
    },
    #[error("Could not serialize the ontology terms for {query}: {source}")]
    Serialize {
        query: String,
        source: serde_json::Error,
    },
}

#[derive(Deserialize)]
pub(crate) struct SearchOntologyArguments {
    query: String,
    limit: Option<usize>,
}

pub(crate) struct SearchOntology {
    pub(super) context: Arc<ToolContext>,
}

impl Tool for SearchOntology {
    const NAME: &'static str = "search_ontology";
    type Args = SearchOntologyArguments;
    type Output = Value;
    type Error = SearchOntologyError;

    fn description(&self) -> String {
        "Search the loaded ontology (e.g. HPO) for terms by ID, name or exact synonym. Returns each term's ID, name and definition.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Term ID prefix or words, e.g. HP:0000252 or microceph" },
                "limit": { "type": "integer", "minimum": 1, "description": "Maximum number of terms (default 100)" }
            },
            "required": ["query"]
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
        let options = search_options(arguments.limit);
        let terms =
            search(&arguments.query, &options).map_err(|source| SearchOntologyError::Search {
                query: arguments.query.clone(),
                source,
            })?;
        let items = serialize_items(&terms).map_err(|source| SearchOntologyError::Serialize {
            query: arguments.query.clone(),
            source,
        })?;
        Ok(item_summary(&self.context, Self::NAME, items))
    }
}

fn search_options(limit: Option<usize>) -> SearchOptions {
    let defaults = SearchOptions::default();
    SearchOptions {
        limit: limit.unwrap_or(defaults.limit),
        ..defaults
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::{call_tool, test_context};

    fn mini_obo_path() -> String {
        let relative = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../seqa_obos/tests/fixtures/mini.obo"
        );
        let canonical = std::fs::canonicalize(relative).unwrap();
        canonical.display().to_string()
    }

    fn ontology_arguments(query: &str, limit: Option<usize>) -> SearchOntologyArguments {
        SearchOntologyArguments {
            query: query.to_string(),
            limit,
        }
    }

    // One ordered test, because seqa_obos holds one ontology per process and the
    // not-loaded check must run before any load.
    #[tokio::test]
    async fn search_before_and_after_loading_the_ontology() {
        let (context, mut event_receiver) = test_context(20, true);
        let tool = SearchOntology { context };

        let not_loaded = call_tool(&tool, ontology_arguments("microceph", None))
            .await
            .unwrap_err();
        assert!(
            not_loaded
                .to_string()
                .contains("no ontology has been loaded"),
            "{not_loaded}"
        );
        assert!(event_receiver.try_recv().is_err());

        let location = mini_obo_path();
        seqa_obos::load(&location).await.unwrap();

        let summary = call_tool(&tool, ontology_arguments("microceph", None))
            .await
            .unwrap();
        let event = event_receiver.try_recv().unwrap();
        assert_eq!(event.items[0]["id"], "HP:0000252");
        assert_eq!(event.items[0]["name"], "Microcephaly");
        assert_eq!(summary["items"][0]["id"], "HP:0000252");

        let zero_limit = call_tool(&tool, ontology_arguments("microceph", Some(0)))
            .await
            .unwrap_err();
        assert!(
            zero_limit.to_string().contains("limit must be at least 1"),
            "{zero_limit}"
        );
    }
}

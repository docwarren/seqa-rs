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

use rig_agent::Agent;
use rig_agent::client::AgentClientExt;
use rig_agent::completion::{Chat, PromptError};
use rig_core::client::Nothing;
use rig_core::http_client;
use rig_core::message::Message;
use rig_core::providers::ollama;
use thiserror::Error;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use crate::config::AgentConfig;
use crate::events::ToolResultEvent;
use crate::tools::{ToolContext, add_built_in_tools};

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

const MAX_TURNS: usize = 10;

const DEFAULT_PREAMBLE: &str = "You are seqa's genomics assistant. You answer questions by calling the seqa tools: \
search_file searches a genomic file for regions, list_directory and list_buckets browse storage, \
supported_formats lists the file formats seqa reads, gene_coordinates and cytobands give reference data, \
and search_ontology searches the loaded ontology (e.g. HPO). \
Coordinates are one-based and written chr:begin-end, e.g. chr17:43044295-43170327, or a whole chromosome such as chr17. \
Tool results are summaries: the full result is shown to the user separately, so you see only counts or the first few items. \
Do not invent data you did not receive from a tool.";

/// Errors from creating or chatting with an [`AgentSession`].
///
/// Tool failures are not session errors: they are sent back to the model as text, and the
/// turn continues.
#[derive(Debug, Error)]
pub enum AgentError {
    /// The Ollama client could not be built for the configured base URL.
    #[error("Could not create the Ollama client for {url}: {source}")]
    Client {
        /// The Ollama base URL.
        url: String,
        /// The underlying client error.
        source: http_client::Error,
    },
    /// A chat turn failed, for example because Ollama could not be reached.
    #[error("Chat failed: {0}")]
    Chat(#[from] PromptError),
}

/// A multi-turn conversation with an Ollama model that can call the seqa tools.
///
/// The session keeps the conversation history in memory and never persists it.
pub struct AgentSession {
    agent: Agent,
    history: Vec<Message>,
}

impl AgentSession {
    /// Creates a session from `config` and returns it with the receiver for tool result
    /// events.
    ///
    /// Every successful tool call sends one [`ToolResultEvent`] holding the full result.
    /// The host should drain the receiver or drop it; sends to a dropped receiver are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`AgentError::Client`] if the Ollama client cannot be built.
    pub fn new(
        config: AgentConfig,
    ) -> Result<(AgentSession, UnboundedReceiver<ToolResultEvent>), AgentError> {
        let (event_sender, event_receiver) = unbounded_channel();
        let context =
            ToolContext::new(config.summary_limit, config.allow_local_paths, event_sender);
        let shared_context = Arc::new(context);
        let agent = build_agent(config, &shared_context)?;
        let session = AgentSession {
            agent,
            history: Vec::new(),
        };
        Ok((session, event_receiver))
    }

    /// Sends one user message and returns the model's final reply.
    ///
    /// The model may call several tools before it replies. The message and reply are added
    /// to the history only when the turn succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`AgentError::Chat`] if the turn fails, for example when Ollama is unreachable.
    pub async fn chat(&mut self, message: &str) -> Result<String, AgentError> {
        let reply = self.agent.chat(message, &mut self.history).await?;
        Ok(reply)
    }

    /// Clears the conversation history. The tools and configuration are kept.
    pub fn reset(&mut self) {
        self.history.clear();
    }
}

fn build_agent(config: AgentConfig, context: &Arc<ToolContext>) -> Result<Agent, AgentError> {
    let client = ollama_client(&config)?;
    let preamble = config.preamble.as_deref().unwrap_or(DEFAULT_PREAMBLE);
    let builder = client
        .agent(config.model)
        .preamble(preamble)
        .default_max_turns(MAX_TURNS);
    let builder = add_built_in_tools(builder, context);
    let agent = builder.dynamic_tools(config.extra_tools).build();
    Ok(agent)
}

fn ollama_client(config: &AgentConfig) -> Result<ollama::Client, AgentError> {
    let url = config.ollama_url.as_deref().unwrap_or(DEFAULT_OLLAMA_URL);
    ollama::Client::builder()
        .api_key(Nothing)
        .base_url(url)
        .build()
        .map_err(|source| AgentError::Client {
            url: url.to_string(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_agent::tool::{DynamicTool, ToolOutput};
    use serde_json::json;

    const BUILT_IN_TOOL_NAMES: [&str; 7] = [
        "cytobands",
        "gene_coordinates",
        "list_buckets",
        "list_directory",
        "search_file",
        "search_ontology",
        "supported_formats",
    ];

    fn open_track_tool() -> DynamicTool {
        DynamicTool::new(
            "open_track",
            "Open a track in the viewer",
            json!({ "type": "object", "properties": {} }),
            |_context, _arguments| Box::pin(async { Ok(ToolOutput::text("opened")) }),
        )
    }

    async fn sorted_tool_names(session: &AgentSession) -> Vec<String> {
        let definitions = session.agent.tool_definitions(None).await.unwrap();
        let mut names: Vec<String> = definitions
            .into_iter()
            .map(|definition| definition.name)
            .collect();
        names.sort();
        names
    }

    fn unreachable_config() -> AgentConfig {
        let mut config = AgentConfig::new("llama3.1");
        config.ollama_url = Some("http://127.0.0.1:9".to_string());
        config
    }

    #[tokio::test]
    async fn session_offers_exactly_the_seven_built_in_tools() {
        let config = AgentConfig::new("llama3.1");
        let (session, _event_receiver) = AgentSession::new(config).unwrap();
        let names = sorted_tool_names(&session).await;
        assert_eq!(names, BUILT_IN_TOOL_NAMES);
    }

    #[tokio::test]
    async fn session_offers_extra_tools_alongside_the_built_in_tools() {
        let mut config = AgentConfig::new("llama3.1");
        config.extra_tools.push(open_track_tool());
        let (session, _event_receiver) = AgentSession::new(config).unwrap();
        let names = sorted_tool_names(&session).await;
        let mut expected: Vec<&str> = BUILT_IN_TOOL_NAMES.to_vec();
        expected.push("open_track");
        expected.sort();
        assert_eq!(names, expected);
    }

    #[tokio::test]
    async fn unreachable_ollama_returns_an_error_and_keeps_history_empty() {
        let (mut session, _event_receiver) = AgentSession::new(unreachable_config()).unwrap();
        let result = session.chat("what cytobands are on chr17 in hg38?").await;
        assert!(matches!(result, Err(AgentError::Chat(_))));
        assert!(session.history.is_empty());
    }

    #[test]
    fn default_ollama_url_is_local() {
        let config = AgentConfig::new("llama3.1");
        let client = ollama_client(&config).unwrap();
        assert_eq!(client.base_url(), "http://localhost:11434");
    }

    #[test]
    fn custom_ollama_url_is_used() {
        let mut config = AgentConfig::new("llama3.1");
        config.ollama_url = Some("http://gpu-box:11434".to_string());
        let client = ollama_client(&config).unwrap();
        assert_eq!(client.base_url(), "http://gpu-box:11434");
    }

    #[test]
    fn reset_clears_the_history() {
        let (mut session, _event_receiver) = AgentSession::new(unreachable_config()).unwrap();
        session.history.push(Message::user("hello"));
        session.reset();
        assert!(session.history.is_empty());
    }
}

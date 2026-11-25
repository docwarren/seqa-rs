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

use rig_agent::tool::DynamicTool;

const DEFAULT_SUMMARY_LIMIT: usize = 20;

/// Settings for an [`AgentSession`](crate::AgentSession).
///
/// Build one with [`AgentConfig::new`] and change any field before passing it to
/// [`AgentSession::new`](crate::AgentSession::new).
pub struct AgentConfig {
    /// Ollama model name, e.g. `llama3.1`. The model must support tool calling.
    pub model: String,
    /// Ollama base URL. `None` uses `http://localhost:11434`.
    pub ollama_url: Option<String>,
    /// Maximum number of items a tool returns to the model. The host still receives every item.
    pub summary_limit: usize,
    /// Whether `search_file` and `list_directory` accept local paths and `file:` URIs.
    /// The model cannot change this.
    pub allow_local_paths: bool,
    /// System prompt. `None` uses a built-in prompt that describes the seqa tools.
    pub preamble: Option<String>,
    /// Host-supplied tools, offered to the model alongside the built-in tools.
    pub extra_tools: Vec<DynamicTool>,
}

impl AgentConfig {
    /// Creates a configuration for `model` with the defaults: local Ollama, a summary limit
    /// of 20 items, local paths allowed, the built-in preamble and no extra tools.
    pub fn new(model: &str) -> Self {
        AgentConfig {
            model: model.to_string(),
            ollama_url: None,
            summary_limit: DEFAULT_SUMMARY_LIMIT,
            allow_local_paths: true,
            preamble: None,
            extra_tools: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_the_defaults() {
        let config = AgentConfig::new("llama3.1");
        assert_eq!(config.model, "llama3.1");
        assert_eq!(config.ollama_url, None);
        assert_eq!(config.summary_limit, 20);
        assert!(config.allow_local_paths);
        assert_eq!(config.preamble, None);
        assert!(config.extra_tools.is_empty());
    }
}

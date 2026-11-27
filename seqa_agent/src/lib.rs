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

//! A local Ollama-backed agent that answers genomics questions by calling seqa tools.
//!
//! Create an [`AgentSession`] from an [`AgentConfig`], drain the [`ToolResultEvent`]
//! receiver it returns, and send messages with [`AgentSession::chat`]:
//!
//! ```no_run
//! use seqa_agent::{AgentConfig, AgentSession};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut config = AgentConfig::new("llama3.1");
//!     config.allow_local_paths = false;
//!     let (mut session, mut events) = AgentSession::new(config)?;
//!
//!     tokio::spawn(async move {
//!         while let Some(event) = events.recv().await {
//!             println!("[{} {}] {} items", event.tool, event.result_id, event.items.len());
//!         }
//!     });
//!
//!     let reply = session.chat("What cytobands are on chr17 in hg38?").await?;
//!     println!("{reply}");
//!     let follow_up = session.chat("Which of those is 17q21.31?").await?;
//!     println!("{follow_up}");
//!     session.reset();
//!     Ok(())
//! }
//! ```

mod config;
mod events;
mod session;
mod summary;
mod tools;

/// The rig agent runtime this crate is built on. Hosts use it to build extra tools
/// ([`rig_agent::tool::DynamicTool`]) for [`AgentConfig::extra_tools`] with a matching version.
pub use rig_agent;

/// The rig core crate this crate is built on. [`AgentError::Client`] wraps its
/// [`rig_core::http_client::Error`].
pub use rig_core;

pub use config::AgentConfig;
pub use events::ToolResultEvent;
pub use session::{AgentError, AgentSession};

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

//! Needs a running Ollama at http://localhost:11434 with a tool-calling model named in
//! `SEQA_AGENT_TEST_MODEL`. Run with `cargo test -p seqa_agent -- --ignored`.

use seqa_agent::{AgentConfig, AgentSession, ToolResultEvent};
use tokio::sync::mpsc::UnboundedReceiver;

fn drain_events(event_receiver: &mut UnboundedReceiver<ToolResultEvent>) -> Vec<ToolResultEvent> {
    let mut events = Vec::new();
    while let Ok(event) = event_receiver.try_recv() {
        events.push(event);
    }
    events
}

#[tokio::test]
#[ignore = "needs a live Ollama and SEQA_AGENT_TEST_MODEL"]
async fn cytobands_event_arrives_before_the_reply() {
    let model = std::env::var("SEQA_AGENT_TEST_MODEL").expect("set SEQA_AGENT_TEST_MODEL");
    let config = AgentConfig::new(&model);
    let (mut session, mut event_receiver) = AgentSession::new(config).unwrap();

    let reply = session
        .chat("Use the cytobands tool to get the cytobands on chr17 in hg38. How many are there?")
        .await
        .unwrap();

    let events = drain_events(&mut event_receiver);
    let cytobands_event = events.iter().find(|event| event.tool == "cytobands");
    assert!(
        cytobands_event.is_some(),
        "no cytobands event; reply: {reply}"
    );
    assert!(!cytobands_event.unwrap().items.is_empty());
}

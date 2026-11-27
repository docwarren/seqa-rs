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

use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use seqa_agent::{AgentConfig, AgentSession, ToolResultEvent};
use tokio::io::{AsyncBufReadExt, BufReader, stdin};
use tokio::sync::mpsc::UnboundedReceiver;

/// Chat with a local Ollama model that can call the seqa tools.
///
/// Type a message and press enter. /reset clears the conversation, /quit or end of input exits.
#[derive(Parser)]
#[command(name = "seqa-agent")]
struct Arguments {
    /// Ollama model name. The model must support tool calling, e.g. llama3.1
    #[arg(long)]
    model: String,
    /// Ollama base URL [default: http://localhost:11434]
    #[arg(long)]
    ollama_url: Option<String>,
    /// Ontology (e.g. HPO) to load at startup for search_ontology: a path or s3://, gs://, az://, http(s):// URL
    #[arg(long)]
    ontology: Option<String>,
    /// Maximum number of items a tool returns to the model
    #[arg(long, default_value_t = 20)]
    summary_limit: usize,
}

#[tokio::main]
async fn main() -> ExitCode {
    let arguments = Arguments::parse();
    match run(arguments).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(arguments: Arguments) -> Result<(), Box<dyn Error>> {
    load_ontology(arguments.ontology.as_deref()).await?;
    let config = build_config(arguments);
    let (mut session, mut event_receiver) = AgentSession::new(config)?;
    chat_loop(&mut session, &mut event_receiver).await?;
    Ok(())
}

async fn load_ontology(location: Option<&str>) -> Result<(), seqa_obos::Error> {
    if let Some(location) = location {
        seqa_obos::load(location).await?;
    }
    Ok(())
}

fn build_config(arguments: Arguments) -> AgentConfig {
    let mut config = AgentConfig::new(&arguments.model);
    config.ollama_url = arguments.ollama_url;
    config.summary_limit = arguments.summary_limit;
    config
}

fn print_events(event_receiver: &mut UnboundedReceiver<ToolResultEvent>) {
    while let Ok(event) = event_receiver.try_recv() {
        println!("{}", event_line(&event));
    }
}

fn event_line(event: &ToolResultEvent) -> String {
    format!(
        "[{} {}] {} items",
        event.tool,
        event.result_id,
        event.items.len()
    )
}

async fn chat_loop(
    session: &mut AgentSession,
    event_receiver: &mut UnboundedReceiver<ToolResultEvent>,
) -> io::Result<()> {
    let mut lines = BufReader::new(stdin()).lines();
    loop {
        print_prompt()?;
        let Some(line) = lines.next_line().await? else {
            return Ok(());
        };
        match line.trim() {
            "/quit" => return Ok(()),
            "/reset" => reset(session),
            "" => {}
            message => respond(session, event_receiver, message).await,
        }
    }
}

fn print_prompt() -> io::Result<()> {
    print!("> ");
    io::stdout().flush()
}

fn reset(session: &mut AgentSession) {
    session.reset();
    println!("History cleared.");
}

async fn respond(
    session: &mut AgentSession,
    event_receiver: &mut UnboundedReceiver<ToolResultEvent>,
    message: &str,
) {
    let result = session.chat(message).await;
    print_events(event_receiver);
    match result {
        Ok(reply) => println!("{reply}"),
        Err(error) => eprintln!("error: {error}"),
    }
}

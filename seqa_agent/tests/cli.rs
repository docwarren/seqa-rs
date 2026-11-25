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

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use assert_cmd::Command;
use predicates::prelude::*;

const UNREACHABLE_OLLAMA: &str = "http://127.0.0.1:9";

fn seqa_agent() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("seqa-agent")
}

fn repl(stdin: &str) -> assert_cmd::assert::Assert {
    seqa_agent()
        .args(["--model", "llama3.1", "--ollama-url", UNREACHABLE_OLLAMA])
        .write_stdin(stdin)
        .assert()
}

fn mini_obo_path() -> String {
    let relative = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../seqa_obos/tests/fixtures/mini.obo"
    );
    let canonical = std::fs::canonicalize(relative).unwrap();
    canonical.display().to_string()
}

const CYTOBANDS_TOOL_CALL: &str = r#"{"model":"fake","created_at":"2026-01-01T00:00:00Z","message":{"role":"assistant","content":"","tool_calls":[{"function":{"name":"cytobands","arguments":{"genome":"hg38","chromosome":"chr17"}}}]},"done":true,"done_reason":"stop"}"#;
const FINAL_REPLY: &str = r#"{"model":"fake","created_at":"2026-01-01T00:00:00Z","message":{"role":"assistant","content":"chr17 has 24 bands."},"done":true,"done_reason":"stop"}"#;

fn fake_ollama(responses: Vec<&'static str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for response in responses {
            let (stream, _) = listener.accept().unwrap();
            answer_request(stream, response);
        }
    });
    format!("http://{address}")
}

fn answer_request(mut stream: TcpStream, response: &str) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let content_length = read_content_length(&mut reader);
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
        response.len()
    );
    stream.write_all(reply.as_bytes()).unwrap();
}

fn read_content_length(reader: &mut BufReader<TcpStream>) -> usize {
    let mut content_length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let header = line.trim_end().to_ascii_lowercase();
        if header.is_empty() {
            return content_length;
        }
        if let Some(value) = header.strip_prefix("content-length:") {
            content_length = value.trim().parse().unwrap();
        }
    }
}

#[test]
fn tool_events_print_before_the_reply() {
    let url = fake_ollama(vec![CYTOBANDS_TOOL_CALL, FINAL_REPLY]);
    let assert = seqa_agent()
        .args(["--model", "fake", "--ollama-url", &url])
        .write_stdin("what cytobands are on chr17?\n/quit\n")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let event_position = stdout.find("[cytobands r1] 24 items\n").expect(&stdout);
    let reply_position = stdout.find("chr17 has 24 bands.").expect(&stdout);
    assert!(event_position < reply_position, "{stdout}");
}

#[test]
fn missing_model_is_a_usage_error() {
    seqa_agent()
        .assert()
        .failure()
        .stderr(predicate::str::contains("--model"));
}

#[test]
fn missing_ontology_reports_the_load_error() {
    seqa_agent()
        .args(["--model", "llama3.1", "--ontology", "/missing/hp.obo"])
        .write_stdin("/quit\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("/missing/hp.obo"));
}

#[test]
fn ontology_loads_before_the_prompt() {
    let ontology = mini_obo_path();
    seqa_agent()
        .args(["--model", "llama3.1", "--ontology", &ontology])
        .write_stdin("/quit\n")
        .assert()
        .success();
}

#[test]
fn quit_exits_zero() {
    repl("/quit\n").success();
}

#[test]
fn end_of_input_exits_zero() {
    repl("").success();
}

#[test]
fn reset_is_confirmed() {
    repl("/reset\n/quit\n")
        .success()
        .stdout(predicate::str::contains("History cleared."));
}

#[test]
fn unreachable_ollama_prints_an_error_and_keeps_prompting() {
    let assert = repl("hello\n/quit\n")
        .success()
        .stderr(predicate::str::contains("error"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout.matches("> ").count(), 2, "{stdout}");
}

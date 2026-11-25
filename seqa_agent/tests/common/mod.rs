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

//! A fake Ollama chat server on localhost. It answers each request with the next queued
//! response and records every request body, so tests can check what the model was sent.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{Value, json};

pub struct FakeResponse {
    status: &'static str,
    body: String,
}

pub struct FakeOllama {
    pub url: String,
    requests: Arc<Mutex<Vec<Value>>>,
}

impl FakeOllama {
    pub fn start(responses: Vec<FakeResponse>) -> FakeOllama {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded_requests = Arc::clone(&requests);
        thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let body = read_request_body(&stream);
                recorded_requests.lock().unwrap().push(body);
                write_response(&mut stream, &response);
            }
        });
        FakeOllama {
            url: format!("http://{address}"),
            requests,
        }
    }

    pub fn requests(&self) -> Vec<Value> {
        self.requests.lock().unwrap().clone()
    }
}

pub fn mini_obo_path() -> String {
    let relative = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../seqa_obos/tests/fixtures/mini.obo"
    );
    let canonical = std::fs::canonicalize(relative).unwrap();
    canonical.display().to_string()
}

pub fn tool_calls(calls: &[(&str, Value)]) -> FakeResponse {
    let calls: Vec<Value> = calls
        .iter()
        .map(|(name, arguments)| json!({ "function": { "name": name, "arguments": arguments } }))
        .collect();
    let message = json!({ "role": "assistant", "content": "", "tool_calls": calls });
    chat_response(message)
}

pub fn reply(text: &str) -> FakeResponse {
    let message = json!({ "role": "assistant", "content": text });
    chat_response(message)
}

pub fn server_error() -> FakeResponse {
    FakeResponse {
        status: "500 Internal Server Error",
        body: json!({ "error": "fake failure" }).to_string(),
    }
}

fn chat_response(message: Value) -> FakeResponse {
    let body = json!({
        "model": "fake",
        "created_at": "2026-01-01T00:00:00Z",
        "message": message,
        "done": true,
        "done_reason": "stop",
    });
    FakeResponse {
        status: "200 OK",
        body: body.to_string(),
    }
}

fn read_request_body(stream: &TcpStream) -> Value {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let content_length = read_content_length(&mut reader);
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn write_response(stream: &mut TcpStream, response: &FakeResponse) {
    let reply = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.status,
        response.body.len(),
        response.body
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

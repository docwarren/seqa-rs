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

#![cfg(feature = "cli")]

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

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

//! Drives `AgentSession` through its public API against a fake Ollama server, checking
//! what the model is sent and what the host receives.

mod common;

use std::sync::{Arc, Mutex};

use seqa_agent::rig_agent::tool::{DynamicTool, ToolOutput};
use seqa_agent::{AgentConfig, AgentError, AgentSession, ToolResultEvent};
use seqa_core::api::search::search_features;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::stores::StoreService;
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedReceiver;

use common::{FakeOllama, FakeResponse, mini_obo_path, reply, server_error, tool_calls};

const VCF_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../seqa_core/mock_data/NA12878.gatk.cnv.vcf.gz"
);
const MOCK_DATA_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../seqa_core/mock_data/");

const BUILT_IN_TOOL_NAMES: [&str; 7] = [
    "cytobands",
    "gene_coordinates",
    "list_buckets",
    "list_directory",
    "search_file",
    "search_ontology",
    "supported_formats",
];

const COUNT_KEYS: [&str; 3] = ["record_count", "result_id", "tool"];
const SEARCH_FILE_KEYS: [&str; 4] = ["header_line_count", "record_count", "result_id", "tool"];
const ITEM_KEYS: [&str; 5] = ["items", "omitted", "result_id", "tool", "total"];

const FIRST_QUESTION: &str = "what cytobands are on chr17 in hg38?";
const FIRST_REPLY: &str = "chr17 has 24 bands.";
const SECOND_QUESTION: &str = "which of those is 17q21.31?";
const THIRD_QUESTION: &str = "and on chr1?";

fn fake_config(fake_ollama: &FakeOllama) -> AgentConfig {
    let mut config = AgentConfig::new("fake");
    config.ollama_url = Some(fake_ollama.url.clone());
    config
}

fn start_session(config: AgentConfig) -> (AgentSession, UnboundedReceiver<ToolResultEvent>) {
    AgentSession::new(config).unwrap()
}

fn drain_events(event_receiver: &mut UnboundedReceiver<ToolResultEvent>) -> Vec<ToolResultEvent> {
    let mut events = Vec::new();
    while let Ok(event) = event_receiver.try_recv() {
        events.push(event);
    }
    events
}

fn messages(request: &Value) -> &Vec<Value> {
    request["messages"].as_array().unwrap()
}

fn messages_with_role<'a>(request: &'a Value, role: &str) -> Vec<&'a Value> {
    messages(request)
        .iter()
        .filter(|message| message["role"] == role)
        .collect()
}

fn message_content(message: &Value) -> String {
    message["content"].as_str().unwrap().to_string()
}

fn tool_message_contents(request: &Value) -> Vec<String> {
    let tool_messages = messages_with_role(request, "tool");
    tool_messages.into_iter().map(message_content).collect()
}

fn tool_summaries(request: &Value) -> Vec<Value> {
    let contents = tool_message_contents(request);
    contents
        .iter()
        .map(|content| serde_json::from_str(content).expect(content))
        .collect()
}

fn user_questions(request: &Value) -> Vec<String> {
    let user_messages = messages_with_role(request, "user");
    user_messages.into_iter().map(message_content).collect()
}

fn assistant_texts(request: &Value) -> Vec<String> {
    let assistant_messages = messages_with_role(request, "assistant");
    assistant_messages
        .into_iter()
        .map(message_content)
        .filter(|content| !content.is_empty())
        .collect()
}

fn sorted_keys(summary: &Value) -> Vec<String> {
    let mut keys: Vec<String> = summary.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    keys
}

fn offered_tool_names(request: &Value) -> Vec<String> {
    let tools = request["tools"].as_array().unwrap();
    let mut names: Vec<String> = tools
        .iter()
        .map(|tool| tool["function"]["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

fn event_tools(events: &[ToolResultEvent]) -> Vec<&str> {
    events.iter().map(|event| event.tool.as_str()).collect()
}

fn event_result_ids(events: &[ToolResultEvent]) -> Vec<&str> {
    events.iter().map(|event| event.result_id.as_str()).collect()
}

fn summary_result_ids(summaries: &[Value]) -> Vec<&str> {
    summaries
        .iter()
        .map(|summary| summary["result_id"].as_str().unwrap())
        .collect()
}

fn item_lines(event: &ToolResultEvent, kind: &str) -> Vec<String> {
    event
        .items
        .iter()
        .filter(|item| item["kind"] == kind)
        .map(|item| item["line"].as_str().unwrap().to_string())
        .collect()
}

fn search_file_call(path: &str, coordinates: &[&str]) -> (&'static str, Value) {
    ("search_file", json!({ "path": path, "coordinates": coordinates }))
}

fn cytobands_call() -> (&'static str, Value) {
    ("cytobands", json!({ "genome": "hg38", "chromosome": "chr17" }))
}

fn list_directory_call(uri: &str) -> (&'static str, Value) {
    ("list_directory", json!({ "uri": uri }))
}

fn tool_turn_then_reply(calls: &[(&str, Value)]) -> Vec<FakeResponse> {
    vec![tool_calls(calls), reply(FIRST_REPLY)]
}

async fn expected_vcf_lines(coordinates: &[&str]) -> (Vec<String>, Vec<String>) {
    let coordinates: Vec<String> = coordinates
        .iter()
        .map(|coordinate| coordinate.to_string())
        .collect();
    let options = SearchOptions::new(VCF_PATH, &coordinates).unwrap();
    let store_service = StoreService::new();
    let result = search_features(&store_service, &options).await.unwrap();
    let header = result.header.as_ref().unwrap().to_lines();
    let records = result.lines.concat();
    (header, records)
}

fn recording_tool(name: &str, calls: Arc<Mutex<Vec<Value>>>) -> DynamicTool {
    DynamicTool::new(
        name,
        "Open a track in the viewer",
        json!({ "type": "object", "properties": { "uri": { "type": "string" } } }),
        move |_context, arguments| {
            calls.lock().unwrap().push(arguments);
            Box::pin(async { Ok(ToolOutput::text("track opened")) })
        },
    )
}

#[tokio::test]
async fn two_tool_calls_send_two_events_in_call_order_with_matching_ids() {
    let calls = [search_file_call(VCF_PATH, &["chr1:30000-50000"]), cytobands_call()];
    let fake_ollama = FakeOllama::start(tool_turn_then_reply(&calls));
    let (mut session, mut event_receiver) = start_session(fake_config(&fake_ollama));

    let answer = session.chat(FIRST_QUESTION).await.unwrap();

    assert_eq!(answer, FIRST_REPLY);
    let events = drain_events(&mut event_receiver);
    assert_eq!(event_tools(&events), ["search_file", "cytobands"]);
    let requests = fake_ollama.requests();
    let summaries = tool_summaries(&requests[1]);
    assert_eq!(summary_result_ids(&summaries), event_result_ids(&events));
    assert_eq!(sorted_keys(&summaries[0]), SEARCH_FILE_KEYS);
    assert_eq!(sorted_keys(&summaries[1]), COUNT_KEYS);
    assert_eq!(summaries[1]["record_count"], events[1].items.len());
}

#[tokio::test]
async fn item_summaries_are_capped_at_the_summary_limit() {
    seqa_obos::load(&mini_obo_path()).await.unwrap();
    let calls = [
        list_directory_call(MOCK_DATA_DIRECTORY),
        ("supported_formats", json!({})),
        ("search_ontology", json!({ "query": "microceph" })),
        ("gene_coordinates", json!({ "genome": "hg38", "source": "refseq" })),
    ];
    let fake_ollama = FakeOllama::start(tool_turn_then_reply(&calls));
    let mut config = fake_config(&fake_ollama);
    config.summary_limit = 2;
    let (mut session, mut event_receiver) = start_session(config);

    session.chat(FIRST_QUESTION).await.unwrap();

    let events = drain_events(&mut event_receiver);
    let requests = fake_ollama.requests();
    let summaries = tool_summaries(&requests[1]);
    assert_eq!(summaries.len(), 4);
    for (summary, event) in summaries[..3].iter().zip(&events) {
        assert_eq!(sorted_keys(summary), ITEM_KEYS, "{summary}");
        assert_eq!(summary["total"], event.items.len(), "{summary}");
        assert_eq!(summary["items"].as_array().unwrap(), &event.items[..2], "{summary}");
        assert_eq!(summary["omitted"], event.items.len() - 2, "{summary}");
    }
    assert_eq!(sorted_keys(&summaries[3]), COUNT_KEYS);
    assert_eq!(summaries[3]["record_count"], events[3].items.len());
}

#[tokio::test]
async fn list_buckets_summary_holds_bucket_names() {
    let calls = [("list_buckets", json!({ "backend": "s3" }))];
    let fake_ollama = FakeOllama::start(tool_turn_then_reply(&calls));
    let (mut session, mut event_receiver) = start_session(fake_config(&fake_ollama));

    session.chat(FIRST_QUESTION).await.unwrap();

    let events = drain_events(&mut event_receiver);
    let requests = fake_ollama.requests();
    let summaries = tool_summaries(&requests[1]);
    assert_eq!(sorted_keys(&summaries[0]), ITEM_KEYS, "{}", summaries[0]);
    assert_eq!(summaries[0]["total"], events[0].items.len());
    assert!(events[0].items.iter().all(Value::is_string));
}

#[tokio::test]
async fn search_file_sends_every_line_to_the_host_and_none_to_the_model() {
    let coordinates = ["chr1", "chr2"];
    let calls = [search_file_call(VCF_PATH, &coordinates)];
    let fake_ollama = FakeOllama::start(tool_turn_then_reply(&calls));
    let (mut session, mut event_receiver) = start_session(fake_config(&fake_ollama));

    session.chat(FIRST_QUESTION).await.unwrap();

    let events = drain_events(&mut event_receiver);
    let (expected_header, expected_records) = expected_vcf_lines(&coordinates).await;
    assert!(!expected_records.is_empty());
    assert_eq!(item_lines(&events[0], "header"), expected_header);
    assert_eq!(item_lines(&events[0], "record"), expected_records);
    let all_lines = [expected_header, expected_records].concat();
    for request in fake_ollama.requests() {
        let body = request.to_string();
        for line in &all_lines {
            let encoded_line = json!(line).to_string();
            let line_text = encoded_line.trim_matches('"');
            assert!(!body.contains(line_text), "model was sent {line}");
        }
    }
}

#[tokio::test]
async fn disallowed_local_paths_are_rejected_through_the_session() {
    let calls = [
        search_file_call(VCF_PATH, &["chr1:30000-50000"]),
        list_directory_call(MOCK_DATA_DIRECTORY),
    ];
    let fake_ollama = FakeOllama::start(tool_turn_then_reply(&calls));
    let mut config = fake_config(&fake_ollama);
    config.allow_local_paths = false;
    let (mut session, mut event_receiver) = start_session(config);

    let answer = session.chat(FIRST_QUESTION).await.unwrap();

    assert_eq!(answer, FIRST_REPLY);
    assert!(drain_events(&mut event_receiver).is_empty());
    let requests = fake_ollama.requests();
    let contents = tool_message_contents(&requests[1]);
    assert_eq!(contents.len(), 2);
    assert!(contents[0].contains("Local paths are not allowed"), "{}", contents[0]);
    assert!(contents[0].contains(VCF_PATH), "{}", contents[0]);
    assert!(contents[1].contains("Local paths are not allowed"), "{}", contents[1]);
}

#[tokio::test]
async fn host_supplied_tool_is_offered_and_invoked() {
    let recorded_calls = Arc::new(Mutex::new(Vec::new()));
    let calls = [("open_track", json!({ "uri": "s3://bucket/sample.bam" }))];
    let fake_ollama = FakeOllama::start(tool_turn_then_reply(&calls));
    let mut config = fake_config(&fake_ollama);
    let open_track = recording_tool("open_track", Arc::clone(&recorded_calls));
    config.extra_tools.push(open_track);
    let (mut session, mut event_receiver) = start_session(config);

    session.chat(FIRST_QUESTION).await.unwrap();

    let requests = fake_ollama.requests();
    assert!(offered_tool_names(&requests[0]).contains(&"open_track".to_string()));
    assert_eq!(
        *recorded_calls.lock().unwrap(),
        [json!({ "uri": "s3://bucket/sample.bam" })]
    );
    assert_eq!(tool_message_contents(&requests[1]), ["track opened"]);
    assert!(drain_events(&mut event_receiver).is_empty());
}

#[tokio::test]
async fn exactly_the_seven_built_in_tools_are_offered() {
    let fake_ollama = FakeOllama::start(vec![reply(FIRST_REPLY)]);
    let (mut session, _event_receiver) = start_session(fake_config(&fake_ollama));

    session.chat(FIRST_QUESTION).await.unwrap();

    let requests = fake_ollama.requests();
    assert_eq!(offered_tool_names(&requests[0]), BUILT_IN_TOOL_NAMES);
}

#[tokio::test]
async fn extra_tools_are_offered_alongside_the_built_in_tools() {
    let fake_ollama = FakeOllama::start(vec![reply(FIRST_REPLY)]);
    let mut config = fake_config(&fake_ollama);
    let open_track = recording_tool("open_track", Arc::new(Mutex::new(Vec::new())));
    config.extra_tools.push(open_track);
    let (mut session, _event_receiver) = start_session(config);

    session.chat(FIRST_QUESTION).await.unwrap();

    let requests = fake_ollama.requests();
    let mut expected: Vec<&str> = BUILT_IN_TOOL_NAMES.to_vec();
    expected.push("open_track");
    expected.sort();
    assert_eq!(offered_tool_names(&requests[0]), expected);
}

#[tokio::test]
async fn configured_model_and_preamble_are_sent() {
    let fake_ollama = FakeOllama::start(vec![reply(FIRST_REPLY)]);
    let mut config = fake_config(&fake_ollama);
    config.model = "qwen3:8b".to_string();
    config.preamble = Some("You are a test preamble.".to_string());
    let (mut session, _event_receiver) = start_session(config);

    session.chat(FIRST_QUESTION).await.unwrap();

    let requests = fake_ollama.requests();
    assert_eq!(requests[0]["model"], "qwen3:8b");
    let system_messages = messages_with_role(&requests[0], "system");
    assert_eq!(system_messages.len(), 1);
    assert_eq!(message_content(system_messages[0]), "You are a test preamble.");
}

#[tokio::test]
async fn follow_up_includes_the_earlier_turn() {
    let fake_ollama = FakeOllama::start(vec![reply(FIRST_REPLY), reply("17q21.31 is one.")]);
    let (mut session, _event_receiver) = start_session(fake_config(&fake_ollama));

    session.chat(FIRST_QUESTION).await.unwrap();
    session.chat(SECOND_QUESTION).await.unwrap();

    let requests = fake_ollama.requests();
    assert_eq!(user_questions(&requests[1]), [FIRST_QUESTION, SECOND_QUESTION]);
    assert_eq!(assistant_texts(&requests[1]), [FIRST_REPLY]);
}

#[tokio::test]
async fn reset_sends_no_earlier_turns() {
    let fake_ollama = FakeOllama::start(vec![reply(FIRST_REPLY), reply("17q21.31 is one.")]);
    let (mut session, _event_receiver) = start_session(fake_config(&fake_ollama));

    session.chat(FIRST_QUESTION).await.unwrap();
    session.reset();
    session.chat(SECOND_QUESTION).await.unwrap();

    let requests = fake_ollama.requests();
    assert_eq!(user_questions(&requests[1]), [SECOND_QUESTION]);
    assert!(assistant_texts(&requests[1]).is_empty());
}

#[tokio::test]
async fn server_error_returns_an_error_and_leaves_history_unchanged() {
    let responses = vec![reply(FIRST_REPLY), server_error(), reply("chr1 has 63 bands.")];
    let fake_ollama = FakeOllama::start(responses);
    let (mut session, _event_receiver) = start_session(fake_config(&fake_ollama));

    session.chat(FIRST_QUESTION).await.unwrap();
    let failed = session.chat(SECOND_QUESTION).await;
    session.chat(THIRD_QUESTION).await.unwrap();

    assert!(matches!(failed, Err(AgentError::Chat(_))), "{failed:?}");
    let requests = fake_ollama.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(user_questions(&requests[2]), [FIRST_QUESTION, THIRD_QUESTION]);
    assert_eq!(assistant_texts(&requests[2]), [FIRST_REPLY]);
}

#[tokio::test]
async fn failure_after_a_tool_call_leaves_history_unchanged() {
    let responses = vec![
        reply(FIRST_REPLY),
        tool_calls(&[cytobands_call()]),
        server_error(),
        reply("chr1 has 63 bands."),
    ];
    let fake_ollama = FakeOllama::start(responses);
    let (mut session, _event_receiver) = start_session(fake_config(&fake_ollama));

    session.chat(FIRST_QUESTION).await.unwrap();
    let failed = session.chat(SECOND_QUESTION).await;
    session.chat(THIRD_QUESTION).await.unwrap();

    assert!(matches!(failed, Err(AgentError::Chat(_))), "{failed:?}");
    let requests = fake_ollama.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(user_questions(&requests[3]), [FIRST_QUESTION, THIRD_QUESTION]);
    assert!(tool_message_contents(&requests[3]).is_empty());
}

#[tokio::test]
async fn endless_tool_calls_stop_at_the_turn_limit() {
    let responses: Vec<FakeResponse> = (0..20)
        .map(|_| tool_calls(&[cytobands_call()]))
        .collect();
    let fake_ollama = FakeOllama::start(responses);
    let (mut session, _event_receiver) = start_session(fake_config(&fake_ollama));

    let failed = session.chat(FIRST_QUESTION).await;

    assert!(matches!(failed, Err(AgentError::Chat(_))), "{failed:?}");
    let request_count = fake_ollama.requests().len();
    assert!(request_count <= 11, "{request_count} model calls");
}

#[tokio::test]
async fn dropped_event_receiver_does_not_break_chat() {
    let fake_ollama = FakeOllama::start(tool_turn_then_reply(&[cytobands_call()]));
    let (mut session, event_receiver) = start_session(fake_config(&fake_ollama));
    drop(event_receiver);

    let answer = session.chat(FIRST_QUESTION).await.unwrap();

    assert_eq!(answer, FIRST_REPLY);
    let requests = fake_ollama.requests();
    let summaries = tool_summaries(&requests[1]);
    assert_eq!(sorted_keys(&summaries[0]), COUNT_KEYS);
    assert!(summaries[0]["record_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn concurrent_sessions_keep_their_events_separate() {
    let first_ollama = FakeOllama::start(tool_turn_then_reply(&[cytobands_call()]));
    let supported_formats_call = [("supported_formats", json!({}))];
    let second_ollama = FakeOllama::start(tool_turn_then_reply(&supported_formats_call));
    let (mut first_session, mut first_receiver) = start_session(fake_config(&first_ollama));
    let (mut second_session, mut second_receiver) = start_session(fake_config(&second_ollama));

    let first_chat = first_session.chat(FIRST_QUESTION);
    let second_chat = second_session.chat(FIRST_QUESTION);
    let (first_answer, second_answer) = tokio::join!(first_chat, second_chat);

    first_answer.unwrap();
    second_answer.unwrap();
    let first_events = drain_events(&mut first_receiver);
    let second_events = drain_events(&mut second_receiver);
    assert_eq!(event_tools(&first_events), ["cytobands"]);
    assert_eq!(event_tools(&second_events), ["supported_formats"]);
    assert_eq!(event_result_ids(&first_events), ["r1"]);
    assert_eq!(event_result_ids(&second_events), ["r1"]);
}

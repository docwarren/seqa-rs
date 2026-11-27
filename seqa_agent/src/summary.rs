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

use serde_json::{Map, Value, json};

use crate::events::ToolResultEvent;
use crate::tools::ToolContext;

pub(crate) fn item_summary(context: &ToolContext, tool: &str, items: Vec<Value>) -> Value {
    let total = items.len();
    let shown = first_items(&items, context.summary_limit);
    let omitted = total - shown.len();
    let result_id = send_event(context, tool, items);
    json!({
        "result_id": result_id,
        "tool": tool,
        "total": total,
        "items": shown,
        "omitted": omitted,
    })
}

pub(crate) fn count_summary(
    context: &ToolContext,
    tool: &str,
    items: Vec<Value>,
    record_count: usize,
    extra_counts: &[(&str, usize)],
) -> Value {
    let result_id = send_event(context, tool, items);
    let mut summary = Map::new();
    summary.insert("result_id".to_string(), json!(result_id));
    summary.insert("tool".to_string(), json!(tool));
    summary.insert("record_count".to_string(), json!(record_count));
    for (name, count) in extra_counts {
        summary.insert(name.to_string(), json!(count));
    }
    Value::Object(summary)
}

pub(crate) fn record_count_summary(context: &ToolContext, tool: &str, items: Vec<Value>) -> Value {
    let record_count = items.len();
    count_summary(context, tool, items, record_count, &[])
}

fn first_items(items: &[Value], limit: usize) -> Vec<Value> {
    items.iter().take(limit).cloned().collect()
}

fn send_event(context: &ToolContext, tool: &str, items: Vec<Value>) -> String {
    let result_id = context.next_result_id();
    let event = ToolResultEvent {
        result_id: result_id.clone(),
        tool: tool.to_string(),
        items,
    };
    let _ = context.event_sender.send(event);
    result_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::tests::test_context;

    fn numbered_items(count: usize) -> Vec<Value> {
        (0..count).map(|number| json!(number)).collect()
    }

    #[test]
    fn item_summary_truncates_at_the_limit() {
        let (context, _event_receiver) = test_context(2, true);
        let summary = item_summary(&context, "list_directory", numbered_items(5));
        assert_eq!(
            summary,
            json!({ "result_id": "r1", "tool": "list_directory", "total": 5, "items": [0, 1], "omitted": 3 })
        );
    }

    #[test]
    fn item_summary_returns_a_small_result_whole() {
        let (context, _event_receiver) = test_context(20, true);
        let summary = item_summary(&context, "supported_formats", numbered_items(3));
        assert_eq!(summary["items"], json!([0, 1, 2]));
        assert_eq!(summary["total"], 3);
        assert_eq!(summary["omitted"], 0);
    }

    #[test]
    fn item_summary_sends_every_item_with_the_same_id() {
        let (context, mut event_receiver) = test_context(2, true);
        let summary = item_summary(&context, "list_directory", numbered_items(5));
        let event = event_receiver.try_recv().unwrap();
        assert_eq!(event.result_id, summary["result_id"]);
        assert_eq!(event.tool, "list_directory");
        assert_eq!(event.items, numbered_items(5));
    }

    #[test]
    fn count_summary_holds_counts_and_no_items() {
        let (context, mut event_receiver) = test_context(20, true);
        let summary = count_summary(
            &context,
            "search_file",
            numbered_items(4),
            3,
            &[("header_line_count", 1)],
        );
        assert_eq!(
            summary,
            json!({ "result_id": "r1", "tool": "search_file", "record_count": 3, "header_line_count": 1 })
        );
        let event = event_receiver.try_recv().unwrap();
        assert_eq!(event.result_id, "r1");
        assert_eq!(event.items, numbered_items(4));
    }

    #[test]
    fn dropped_receiver_does_not_fail_the_summary() {
        let (context, event_receiver) = test_context(20, true);
        drop(event_receiver);
        let summary = item_summary(&context, "list_buckets", numbered_items(1));
        assert_eq!(summary["total"], 1);
        let counts = count_summary(&context, "cytobands", numbered_items(1), 1, &[]);
        assert_eq!(counts["record_count"], 1);
    }
}

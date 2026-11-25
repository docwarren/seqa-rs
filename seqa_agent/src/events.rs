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

use serde::Serialize;
use serde_json::Value;

/// The full result of one successful tool call, sent to the host.
///
/// `result_id` matches the ID in the summary the model received for the same call.
#[derive(Debug, Clone, Serialize)]
pub struct ToolResultEvent {
    /// ID shared with the model's summary, e.g. `r3`.
    pub result_id: String,
    /// Name of the tool that produced the result.
    pub tool: String,
    /// Every item in the result, as JSON.
    pub items: Vec<Value>,
}

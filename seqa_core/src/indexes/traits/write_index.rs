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

use std::fs::File;

pub trait WriteIndex {
    fn get_keys(&self) -> Vec<String>;
    fn write_size(&self, file_out: &mut File) -> Result<(), String>;
    fn write_header(&self, file_out: &mut File) -> Result<(), String>;
    fn write_offsets(&self, file_out: &mut File) -> Result<(), String>;
    fn get_byte_count(&self) -> u64;
}
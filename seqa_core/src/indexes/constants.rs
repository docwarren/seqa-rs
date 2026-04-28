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

pub const MAX_BLOCK_SIZE: u64 = 64 * 1024;
pub const MAX_BIN_SIZE: usize = (((1<<18) - 1) / 7) as usize;
pub const LINEAR_BIN_SIZE: u32 = 16384;
pub const BIGWIG_HEADER_SIZE: u64 = 64;
pub const BIGWIG_ZOOM_HEADER_SIZE: u64 = 24;
pub const DEFAULT_ZOOM_PIXELS: f32 = 3000.0;
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
use std::io::{ BufRead, BufReader };

const HG38_CYTOBANDS: &str = "../../../data/hg38_cytobands.tsv";
const HG19_CYTOBANDS: &str = "../../../data/hg19_cytobands.tsv";

pub fn get_cytobands(genome: &str) -> Result<Vec<String>, std::io::Error> {
    let file_path = match genome.to_lowercase().as_str() {
        "hg38" => HG38_CYTOBANDS,
        "hg19" => HG19_CYTOBANDS,
        "grch38" => HG38_CYTOBANDS,
        "grch37" => HG19_CYTOBANDS,
        "ch38" => HG38_CYTOBANDS,
        "ch37" => HG19_CYTOBANDS,
        _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unsupported genome build")),
    };

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let cytobands: Vec<String> = reader.lines().filter_map(Result::ok).collect();
    Ok(cytobands)
}

#[test]
fn test_get_cytobands() {
    let hg38_cytobands = get_cytobands("hg38").unwrap();
    assert!(!hg38_cytobands.is_empty());

    let hg19_cytobands = get_cytobands("hg19").unwrap();
    assert!(!hg19_cytobands.is_empty());

    let invalid_cytobands = get_cytobands("hg37");
    assert!(invalid_cytobands.is_err());
}
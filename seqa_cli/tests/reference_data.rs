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

use std::process::Command;

/// Runs the `seqa` binary with `arguments` and returns its stdout. The temp directory points
/// at a path that does not exist, so any attempt to write reference data to disk would fail.
fn run_seqa(arguments: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_seqa"))
        .args(arguments)
        .env("TMPDIR", "/nonexistent/seqa-test-tmp")
        .output()
        .expect("failed to run seqa");
    assert!(
        output.status.success(),
        "seqa {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("seqa printed invalid UTF-8")
}

#[test]
fn cytobands_prints_one_tab_separated_line_per_band() {
    let stdout = run_seqa(&["cytobands", "chr1"]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty());
    assert_eq!(lines[0], "chr1\t0\t2300000\tp36.33\tgneg");
    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 5, "{line}");
        assert_eq!(fields[0], "chr1", "{line}");
    }
}

#[test]
fn cytobands_for_hg19() {
    let stdout = run_seqa(&["cytobands", "chr1", "-r", "hg19"]);
    assert!(stdout.lines().count() > 0);
    assert!(stdout.lines().all(|line| line.starts_with("chr1\t")));
}

#[test]
fn genes_coordinates_prints_symbol_chromosome_begin_end() {
    let stdout = run_seqa(&["genes", "coordinates", "BRCA1"]);
    let fields: Vec<&str> = stdout.trim_end().split('\t').collect();
    assert_eq!(fields.len(), 4, "{stdout}");
    assert_eq!(fields[0], "BRCA1");
    assert_eq!(fields[1], "chr17");
    let c_begin: u32 = fields[2].parse().unwrap();
    let c_end: u32 = fields[3].parse().unwrap();
    assert!(c_begin < c_end);
}

#[test]
fn genes_coordinates_unknown_gene_prints_nothing() {
    let stdout = run_seqa(&["genes", "coordinates", "NOT_A_REAL_GENE"]);
    assert!(stdout.is_empty());
}

#[test]
fn genes_symbols_prints_one_symbol_per_line() {
    let stdout = run_seqa(&["genes", "symbols", "-r", "hg19", "-s", "gencode"]);
    assert!(stdout.lines().count() >= 40_000);
    assert!(stdout.lines().any(|line| line == "BRCA2"));
    assert!(stdout.lines().all(|line| !line.contains('\t')));
}

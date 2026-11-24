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

const MINI: &str = "./tests/fixtures/mini.obo";
const SHORT_MSG: &str = "Search queries must contain at least 3 letters or digits.";

fn obos() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("obos")
}

fn stdout(args: &[&str]) -> String {
    let out = obos()
        .arg("search")
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(out).unwrap()
}

fn stanzas(out: &str) -> Vec<&str> {
    out.trim_end_matches('\n').split("\n\n").collect()
}

#[test]
fn basic_search() {
    let out = stdout(&[MINI, "microceph"]);
    assert!(out.contains("id: HP:0000252\n"));
    assert!(!out.contains("HP:0000257"));
}

#[test]
fn multi_word_query() {
    let out = stdout(&[MINI, "progressive micr"]);
    for id in ["HP:0000253", "HP:0000254", "HP:0000255"] {
        assert!(out.contains(&format!("id: {id}\n")), "{id}");
    }
    assert!(!out.contains("HP:0000408"));
}

#[test]
fn id_search_prints_term_first() {
    let out = stdout(&[MINI, "hp:0000252"]);
    assert!(
        out.starts_with("[Term]\nid: HP:0000252\nname: Microcephaly\n"),
        "{out}"
    );
}

#[test]
fn stanza_format_and_separation() {
    let out = stdout(&["--limit", "3", MINI, "microcephaly"]);
    assert!(out.ends_with('\n') && !out.ends_with("\n\n"));
    assert!(!out.contains("\n\n\n"), "{out}");
    let s = stanzas(&out);
    assert_eq!(s.len(), 3);
    for st in &s {
        let lines: Vec<&str> = st.lines().collect();
        assert_eq!(lines[0], "[Term]");
        assert!(lines[1].starts_with("id: "));
        assert!(lines[2].starts_with("name: "));
        assert!(lines.iter().all(|l| !l.is_empty()));
    }
    let first: Vec<&str> = s[0].lines().map(|l| l.split(':').next().unwrap()).collect();
    assert_eq!(
        first,
        [
            "[Term]", "id", "name", "alt_id", "def", "synonym", "synonym", "synonym", "synonym",
            "synonym", "xref", "xref", "is_a"
        ]
    );
    assert!(s[0].contains("is_a: HP:0040195 ! Decreased head circumference"));
    assert!(s[0].contains("xref: UMLS:C0025958 {source=\"MONDO\"} ! from MONDO"));
}

#[test]
fn limit_one() {
    assert_eq!(
        stanzas(&stdout(&["--limit", "1", MINI, "microceph"])).len(),
        1
    );
}

#[test]
fn zero_limit_is_usage_error() {
    obos()
        .args(["search", "--limit", "0", MINI, "abn"])
        .assert()
        .code(2)
        .stderr(predicate::str::is_empty().not());
}

#[test]
fn missing_query_is_usage_error() {
    obos()
        .args(["search", MINI])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn json_output() {
    let out = stdout(&["--json", MINI, "microcephaly"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let t = &v.as_array().unwrap()[0];
    assert_eq!(t["id"], "HP:0000252");
    assert_eq!(t["name"], "Microcephaly");
    assert_eq!(t["alt_ids"][0]["id"], "HP:0001366");
    assert!(t["def"]["text"].is_string());
    assert_eq!(t["synonyms"].as_array().unwrap().len(), 5);
    assert_eq!(t["is_a"][0]["comment"], "Decreased head circumference");
}

#[test]
fn short_query_text_mode() {
    // A missing file shows the file is never loaded for a short query.
    obos()
        .args(["search", "./tests/fixtures/missing.obo", "hp"])
        .assert()
        .code(0)
        .stdout(format!("{SHORT_MSG}\n"));
}

#[test]
fn short_query_json_mode() {
    obos()
        .args(["search", "--json", MINI, "hp"])
        .assert()
        .code(0)
        .stdout("[]\n")
        .stderr(predicate::str::contains(SHORT_MSG));
}

#[test]
fn help_lists_ranking() {
    let out = obos()
        .args(["search", "--help"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    let mut pos = 0;
    for phrase in [
        "Exact ID",
        "ID prefix",
        "Exact alt ID",
        "Alt ID prefix",
        "Exact name",
        "Name prefix",
        "Exact synonym",
        "Synonym prefix",
        "Any other word match",
    ] {
        let at = out[pos..]
            .find(phrase)
            .unwrap_or_else(|| panic!("{phrase:?} missing or out of order:\n{out}"));
        pos += at + phrase.len();
    }
    assert!(out.contains("at least 3 letters or digits"));
    assert!(out.contains("--limit"));
}

#[test]
fn no_matches() {
    obos()
        .args(["search", MINI, "zzzzqqq"])
        .assert()
        .code(0)
        .stdout("");
    obos()
        .args(["search", "--json", MINI, "zzzzqqq"])
        .assert()
        .code(0)
        .stdout("[]\n");
}

#[test]
fn missing_file() {
    obos()
        .args(["search", "./tests/fixtures/missing.obo", "microceph"])
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("not found").and(predicate::str::contains("missing.obo")));
}

#[test]
fn synonyms_matched_by_default() {
    let out = stdout(&[MINI, "small head"]);
    assert!(
        out.contains("[Term]\nid: HP:0000252\nname: Microcephaly\n"),
        "{out}"
    );
}

#[test]
fn no_synonyms_flag() {
    let out = stdout(&["--no-synonyms", MINI, "small head"]);
    assert!(!out.contains("HP:0000252"), "{out}");
    assert!(out.contains("id: HP:0040196\n"));
}

#[test]
fn help_mentions_synonyms() {
    let out = obos()
        .args(["search", "--help"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("--no-synonyms"));
    assert!(
        out.contains("EXACT synonyms are matched by default")
            && !out.contains("NARROW synonyms are matched")
            && !out.contains("EXACT before NARROW"),
        "{out}"
    );
}

#[test]
fn query_may_start_with_hyphen() {
    let out = stdout(&[MINI, "-cephaly"]);
    assert!(out.contains("id: HP:0000259\n"), "{out}");
}

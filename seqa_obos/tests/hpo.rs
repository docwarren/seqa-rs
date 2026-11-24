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

//! Real-data tests against a committed HPO release (tests/fixtures/hp.obo). Offline.

use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Instant;

use seqa_obos::{SearchOptions, SynonymScope, Term, search};

const HP: &str = "./tests/fixtures/hp.obo";

/// Loads hp.obo into the global cache once per test binary and prints the load time.
fn ensure_loaded() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let start = Instant::now();
        rt.block_on(seqa_obos::load(HP)).unwrap();
        println!("loaded {HP} in {:?}", start.elapsed());
    });
}

fn obsolete_ids() -> &'static HashSet<String> {
    static IDS: OnceLock<HashSet<String>> = OnceLock::new();
    IDS.get_or_init(|| {
        let doc = fastobo::from_file(HP).unwrap();
        doc.entities()
            .iter()
            .filter_map(|e| e.as_term())
            .filter(|t| {
                t.clauses()
                    .iter()
                    .any(|l| matches!(l.as_inner(), fastobo::ast::TermClause::IsObsolete(true)))
            })
            .map(|t| t.id().as_inner().to_string())
            .collect()
    })
}

fn run(q: &str, opts: &SearchOptions) -> Vec<Term> {
    ensure_loaded();
    search(q, opts).unwrap()
}

fn ids(q: &str) -> Vec<String> {
    run(q, &SearchOptions::default())
        .into_iter()
        .map(|t| t.id)
        .collect()
}

fn assert_no_obsolete(ids: &[String]) {
    let obsolete = obsolete_ids();
    assert!(!obsolete.is_empty());
    for id in ids {
        assert!(!obsolete.contains(id), "obsolete term {id} returned");
    }
}

#[test]
fn microcephaly_first() {
    assert_eq!(ids("microcephaly")[0], "HP:0000252");
}

#[test]
fn microceph_and_progressive_mic() {
    let r = ids("microceph");
    assert!(r.contains(&"HP:0000252".into()));
    assert!(r.contains(&"HP:0000253".into()));
    assert_no_obsolete(&r);

    let r = run("progressive mic", &SearchOptions::default());
    assert!(
        r.iter().any(|t| t.id == "HP:0000253"),
        "Progressive microcephaly missing"
    );
    // Each result's name, or one of its EXACT synonyms, must have a word starting
    // with "progressive" and a word starting with "mic".
    let words_match = |text: &str| {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        ["progressive", "mic"]
            .iter()
            .all(|q| words.iter().any(|w| w.starts_with(q)))
    };
    for t in &r {
        let syn_match = t
            .synonyms
            .iter()
            .any(|s| s.value.scope == SynonymScope::Exact && words_match(&s.value.text));
        assert!(words_match(&t.name) || syn_match, "{} {}", t.id, t.name);
    }
    assert_no_obsolete(&r.into_iter().map(|t| t.id).collect::<Vec<_>>());
}

#[test]
fn id_search_returns_full_entry() {
    let r = run("HP:0000252", &SearchOptions::default());
    let t = &r[0];
    assert_eq!(t.id, "HP:0000252");
    assert_eq!(t.name, "Microcephaly");
    let alts: Vec<&str> = t.alt_ids.iter().map(|a| a.value.id.as_str()).collect();
    assert_eq!(
        alts,
        ["HP:0001366", "HP:0005485", "HP:0005489", "HP:0005497"]
    );
    let def = &t.def.as_ref().unwrap().value;
    assert!(
        def.text
            .starts_with("Head circumference below 2 standard deviations")
    );
    assert_eq!(def.xrefs.len(), 4);
    assert_eq!(t.synonyms.len(), 13);
    assert!(t.comment.is_some());
    let parents: Vec<(&str, Option<&str>)> = t
        .is_a
        .iter()
        .map(|p| (p.value.id.as_str(), p.comment.as_deref()))
        .collect();
    assert_eq!(
        parents,
        [
            ("HP:0007364", Some("Aplasia/Hypoplasia of the cerebrum")),
            ("HP:0040195", Some("Decreased head circumference"))
        ]
    );
}

#[test]
fn alt_id_search() {
    assert_eq!(ids("hp:0001366")[0], "HP:0000252");
}

#[test]
fn exact_synonym_search() {
    assert!(ids("small cranium").contains(&"HP:0000252".into()));
    let off = SearchOptions {
        include_synonyms: false,
        ..Default::default()
    };
    assert!(
        !run("small cranium", &off)
            .iter()
            .any(|t| t.id == "HP:0000252")
    );
}

#[test]
fn short_query_is_empty() {
    assert!(ids("hp").is_empty());
}

#[cfg(feature = "cli")]
mod cli {
    use super::HP;

    fn obos(args: &[&str]) -> String {
        let out = assert_cmd::cargo::cargo_bin_cmd!("obos")
            .arg("search")
            .args(args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn text_output_parses_back_as_obo() {
        let out = obos(&[HP, "microcephaly"]);
        assert!(
            out.starts_with("[Term]\nid: HP:0000252\nname: Microcephaly\n"),
            "{out}"
        );
        let doc = fastobo::from_str(format!("format-version: 1.2\n\n{out}")).unwrap();
        assert!(doc.entities().len() > 1);
        assert!(doc.entities().iter().all(|e| e.as_term().is_some()));
    }

    #[test]
    fn json_output_is_an_array() {
        let out = obos(&["--json", HP, "microcephaly"]);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["id"], "HP:0000252");
        assert_eq!(arr[0]["name"], "Microcephaly");
    }

    #[test]
    fn synonym_flag() {
        assert!(obos(&[HP, "small cranium"]).contains("id: HP:0000252\n"));
        assert!(!obos(&["--no-synonyms", HP, "small cranium"]).contains("id: HP:0000252\n"));
    }
}

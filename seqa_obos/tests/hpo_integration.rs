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

//! Cross-component tests against the committed HPO release (tests/fixtures/hp.obo).
//! Offline. The library side loads hp.obo once into this binary's global cache; the CLI
//! side runs the `obos` binary as a separate process.
//!
//! Two independent oracles are used:
//! - `raw()`: the [Term] stanzas of hp.obo split by hand from the file text, used to check
//!   which clauses (and in which order) appear in JSON and text output.
//! - `oracle_search()`: a reference implementation of the spec's matching and ranking
//!   rules over terms parsed with `fastobo`, used to check the full ranked list.

use std::collections::HashMap;
use std::sync::OnceLock;

use seqa_obos::{SearchOptions, Term, search};
use serde_json::Value as Json;

const HP: &str = "./tests/fixtures/hp.obo";

fn ensure_loaded() {
    static LOADED: OnceLock<()> = OnceLock::new();
    LOADED.get_or_init(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(seqa_obos::load(HP)).unwrap();
    });
}

fn lib(q: &str, include_synonyms: bool, limit: usize) -> Vec<Term> {
    ensure_loaded();
    search(
        q,
        &SearchOptions {
            include_synonyms,
            limit,
        },
    )
    .unwrap()
}

fn lib_ids(q: &str, include_synonyms: bool, limit: usize) -> Vec<String> {
    lib(q, include_synonyms, limit)
        .into_iter()
        .map(|t| t.id)
        .collect()
}

// ---------------------------------------------------------------------------------------
// Raw stanzas from the file text
// ---------------------------------------------------------------------------------------

struct Raw {
    /// Term IDs in file order.
    order: Vec<String>,
    /// Term ID -> (tag, rest of line) for every line after `[Term]`, including `id:`.
    stanzas: HashMap<String, Vec<(String, String)>>,
}

fn raw() -> &'static Raw {
    static RAW: OnceLock<Raw> = OnceLock::new();
    RAW.get_or_init(|| {
        let text = std::fs::read_to_string(HP).unwrap();
        let mut order = Vec::new();
        let mut stanzas = HashMap::new();
        for block in text.split("\n\n") {
            let mut lines = block.lines().filter(|l| !l.is_empty());
            if lines.next() != Some("[Term]") {
                continue;
            }
            let clauses: Vec<(String, String)> = lines
                .map(|l| {
                    let (tag, rest) = l.split_once(": ").unwrap_or_else(|| panic!("{l}"));
                    (tag.to_string(), rest.to_string())
                })
                .collect();
            let id = clauses[0].1.clone();
            assert_eq!(clauses[0].0, "id");
            order.push(id.clone());
            stanzas.insert(id, clauses);
        }
        Raw { order, stanzas }
    })
}

fn raw_stanza(id: &str) -> &'static [(String, String)] {
    raw()
        .stanzas
        .get(id)
        .unwrap_or_else(|| panic!("{id} not in hp.obo"))
}

fn raw_values<'a>(st: &'a [(String, String)], tag: &str) -> Vec<&'a str> {
    st.iter()
        .filter(|(t, _)| t == tag)
        .map(|(_, v)| v.as_str())
        .collect()
}

fn is_searchable(id: &str) -> bool {
    let st = raw_stanza(id);
    st.iter().any(|(t, _)| t == "name")
        && !st.iter().any(|(t, v)| t == "is_obsolete" && v == "true")
}

/// Splits a leading OBO quoted string off `s`: returns (unescaped text, remainder).
fn split_quoted(s: &str) -> (String, &str) {
    let s = s
        .strip_prefix('"')
        .unwrap_or_else(|| panic!("not quoted: {s}"));
    let mut out = String::new();
    let mut chars = s.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                let (_, n) = chars.next().unwrap();
                out.push(n);
            }
            '"' => return (out, s[i + 1..].trim_start()),
            c => out.push(c),
        }
    }
    panic!("unterminated quoted string: {s}");
}

// ---------------------------------------------------------------------------------------
// Reference implementation of the spec's search rules
// ---------------------------------------------------------------------------------------

struct OTerm {
    id: String,
    name: String,
    alt_ids: Vec<String>,
    /// EXACT synonyms only (NARROW, BROAD and RELATED are never matched).
    syns: Vec<String>,
}

fn oracle_terms() -> &'static [OTerm] {
    use fastobo::ast::{SynonymScope as S, TermClause as C};
    static TERMS: OnceLock<Vec<OTerm>> = OnceLock::new();
    TERMS.get_or_init(|| {
        let doc = fastobo::from_file(HP).unwrap();
        doc.entities()
            .iter()
            .filter_map(|e| e.as_term())
            .filter_map(|t| {
                let mut name = None;
                let mut alt_ids = Vec::new();
                let mut syns = Vec::new();
                for l in t.clauses() {
                    match l.as_inner() {
                        C::IsObsolete(true) => return None,
                        C::Name(n) if name.is_none() => name = Some(n.as_str().to_string()),
                        C::AltId(a) => alt_ids.push(a.to_string()),
                        C::Synonym(s) if *s.scope() == S::Exact => {
                            syns.push(s.description().as_str().to_string())
                        }
                        _ => {}
                    }
                }
                Some(OTerm {
                    id: t.id().as_inner().to_string(),
                    name: name?,
                    alt_ids,
                    syns,
                })
            })
            .collect()
    })
}

fn words(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

fn oracle_search(query: &str, include_synonyms: bool) -> Vec<String> {
    if query.chars().filter(|c| c.is_alphanumeric()).count() < 3 {
        return vec![];
    }
    let q = query.trim().to_lowercase();
    let qw = words(query);
    let qn = qw.join(" ");
    let word_match = |text: &str| {
        let tw = words(text);
        !qw.is_empty()
            && qw
                .iter()
                .all(|w| tw.iter().any(|t| t.starts_with(w.as_str())))
    };
    let norm = |text: &str| words(text).join(" ");
    let mut hits: Vec<(u8, String, String)> = Vec::new();
    for t in oracle_terms() {
        let id = t.id.to_lowercase();
        let alts: Vec<String> = t.alt_ids.iter().map(|a| a.to_lowercase()).collect();
        let name_n = norm(&t.name);
        let syns: &[String] = if include_synonyms { &t.syns } else { &[] };
        let any_syn = |pred: &dyn Fn(&str) -> bool| syns.iter().any(|s| pred(&norm(s)));
        let rank = if id == q {
            Some(1)
        } else if id.starts_with(&q) {
            Some(2)
        } else if alts.contains(&q) {
            Some(3)
        } else if alts.iter().any(|a| a.starts_with(&q)) {
            Some(4)
        } else if name_n == qn {
            Some(5)
        } else if name_n.starts_with(&qn) {
            Some(6)
        } else if any_syn(&|s| s == qn) {
            Some(7)
        } else if any_syn(&|s| s.starts_with(&qn)) {
            Some(8)
        } else if word_match(&t.name) || syns.iter().any(|s| word_match(s)) {
            Some(9)
        } else {
            None
        };
        if let Some(r) = rank {
            hits.push((r, t.name.to_lowercase(), t.id.clone()));
        }
    }
    hits.sort();
    hits.into_iter().map(|(_, _, id)| id).collect()
}

// ---------------------------------------------------------------------------------------
// Library tests
// ---------------------------------------------------------------------------------------

const ORACLE_QUERIES: &[&str] = &[
    "microceph",
    "MICROCEPH",
    "microcephaly",
    "progressive mic",
    "mic progressive",
    "abnormal heart",
    "hp:000025",
    "HP:00002",
    "HP:0001366",
    "  HP:0000252  ",
    "small cranium",
    "small head",
    "small head circumference",
    "hypoplasia maxilla",
    "upper jaw",
    "0000252",
    "folie à",
    "FOLIE À DEUX",
    "flat cheek",
    "x-linked",
    "abn",
];

#[test]
fn full_ranking_matches_spec_oracle() {
    for q in ORACLE_QUERIES {
        for syn in [true, false] {
            let got = lib_ids(q, syn, usize::MAX);
            let want = oracle_search(q, syn);
            if got != want {
                let first = got.iter().zip(&want).position(|(a, b)| a != b);
                panic!(
                    "query {q:?} synonyms={syn}: {} results, oracle {}; first difference at {first:?}\n\
                     got[..20]  = {:?}\nwant[..20] = {:?}",
                    got.len(),
                    want.len(),
                    &got[..got.len().min(20)],
                    &want[..want.len().min(20)],
                );
            }
        }
    }
}

#[test]
fn default_and_custom_limits_take_the_top_of_the_ranking() {
    let all = lib_ids("abn", true, usize::MAX);
    assert!(all.len() > 250, "only {} results for abn", all.len());
    ensure_loaded();
    let default: Vec<String> = search("abn", &SearchOptions::default())
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(default, all[..100]);
    assert_eq!(lib_ids("abn", true, 20), all[..20]);
    assert_eq!(lib_ids("abn", true, 1), all[..1]);
}

/// `HP:00002`: no term has that exact ID, so tier 2 (ID prefix) comes first, sorted by name
/// then ID (the "Deterministic order within a tier" requirement), then tier 4 (alt ID
/// prefix) in the same order. Expected lists are computed from the raw file text.
#[test]
fn id_prefix_query_lists_id_matches_first() {
    let q = "HP:00002";
    let r = raw();
    let name_of = |id: &str| raw_values(raw_stanza(id), "name")[0].to_lowercase();
    let mut tier2: Vec<String> = r
        .order
        .iter()
        .filter(|id| id.starts_with(q) && is_searchable(id))
        .cloned()
        .collect();
    tier2.sort_by_key(|id| (name_of(id), id.clone()));
    let mut tier4: Vec<String> = r
        .order
        .iter()
        .filter(|id| !id.starts_with(q) && is_searchable(id))
        .filter(|id| {
            raw_values(raw_stanza(id), "alt_id")
                .iter()
                .any(|a| a.starts_with(q))
        })
        .cloned()
        .collect();
    tier4.sort_by_key(|id| (name_of(id), id.clone()));
    assert!(tier2.len() > 50 && !tier4.is_empty());

    let got = lib_ids(q, true, usize::MAX);
    assert_eq!(got[..tier2.len()], tier2[..], "tier 2 block");
    assert_eq!(
        got[tier2.len()..tier2.len() + tier4.len()],
        tier4[..],
        "tier 4 block"
    );
    // Lowercase query gives the identical list.
    assert_eq!(lib_ids("hp:00002", true, usize::MAX), got);
}

/// IDs checked for JSON content: hand-picked terms with interesting clauses, plus every
/// 400th searchable term in the file.
fn sample_ids() -> Vec<String> {
    let mut ids: Vec<String> = [
        "HP:0000252", // alt_ids, def xrefs, comment, 13 synonyms, xref, two is_a with comments
        "HP:0000327", // EXACT/NARROW/BROAD/RELATED synonyms
        "HP:0000272", // comment with a {xref=...} qualifier, nine alt_ids
        "HP:0000023", // xref with a description
        "HP:0000956", // subset
        "HP:0000001", // root: comment only
        "HP:0000002", // property_value
    ]
    .map(String::from)
    .to_vec();
    ids.extend(
        raw()
            .order
            .iter()
            .filter(|id| is_searchable(id))
            .step_by(400)
            .cloned(),
    );
    ids
}

const LIST_FIELDS: &[(&str, &str)] = &[
    ("alt_id", "alt_ids"),
    ("subset", "subsets"),
    ("synonym", "synonyms"),
    ("xref", "xrefs"),
    ("property_value", "property_values"),
    ("is_a", "is_a"),
    ("intersection_of", "intersection_of"),
    ("union_of", "union_of"),
    ("equivalent_to", "equivalent_to"),
    ("disjoint_from", "disjoint_from"),
    ("relationship", "relationships"),
    ("replaced_by", "replaced_by"),
    ("consider", "consider"),
];
const SINGLE_FIELDS: &[(&str, &str)] = &[
    ("def", "def"),
    ("comment", "comment"),
    ("namespace", "namespace"),
    ("created_by", "created_by"),
    ("creation_date", "creation_date"),
    ("is_anonymous", "is_anonymous"),
    ("builtin", "builtin"),
];

/// Checks the JSON object for one term against its raw stanza.
fn check_term_json(id: &str, t: &Json) {
    let st = raw_stanza(id);
    let obj = t.as_object().unwrap();
    assert_eq!(t["id"], id);
    assert_eq!(t["name"], raw_values(st, "name")[0], "{id}");

    // Only known keys; never is_obsolete.
    for k in obj.keys() {
        let known = k == "id"
            || k == "name"
            || LIST_FIELDS.iter().any(|(_, j)| j == k)
            || SINGLE_FIELDS.iter().any(|(_, j)| j == k);
        assert!(known, "{id}: unexpected key {k}");
    }
    // Field presence and array length follow the clauses in the file.
    for (tag, key) in LIST_FIELDS {
        let n = raw_values(st, tag).len();
        match obj.get(*key) {
            None => assert_eq!(n, 0, "{id}: {key} missing but file has {n} {tag} lines"),
            Some(v) => {
                let len = v.as_array().unwrap().len();
                assert!(n > 0, "{id}: empty {key} present");
                assert_eq!(len, n, "{id}: {key}");
            }
        }
    }
    for (tag, key) in SINGLE_FIELDS {
        let n = raw_values(st, tag).len();
        assert_eq!(obj.contains_key(*key), n > 0, "{id}: {key}");
    }

    // alt_ids in file order.
    let alts: Vec<&str> = raw_values(st, "alt_id");
    for (i, a) in alts.iter().enumerate() {
        assert_eq!(t["alt_ids"][i]["id"], *a, "{id} alt_id {i}");
    }

    // is_a: ID and trailing comment.
    for (i, v) in raw_values(st, "is_a").iter().enumerate() {
        let (pid, comment) = v.split_once(" ! ").unwrap();
        assert_eq!(t["is_a"][i]["id"], pid, "{id} is_a {i}");
        assert_eq!(t["is_a"][i]["comment"], comment, "{id} is_a {i}");
    }

    // Synonyms of every scope, in file order, with text, scope and type.
    for (i, v) in raw_values(st, "synonym").iter().enumerate() {
        let (text, rest) = split_quoted(v);
        let mut parts = rest.split_whitespace();
        let scope = parts.next().unwrap();
        let ty = parts.next().filter(|p| !p.starts_with('['));
        let s = &t["synonyms"][i];
        assert_eq!(s["text"], text.as_str(), "{id} synonym {i}");
        assert_eq!(s["scope"], scope, "{id} synonym {i}");
        match ty {
            Some(ty) => assert_eq!(s["type"], ty, "{id} synonym {i}"),
            None => assert!(s.get("type").is_none(), "{id} synonym {i}"),
        }
        assert!(
            s["xrefs"].is_array(),
            "{id} synonym {i}: xrefs must always be present"
        );
    }

    // def: text and xref IDs.
    if let Some(v) = raw_values(st, "def").first() {
        let (text, rest) = split_quoted(v);
        let inner = rest
            .trim()
            .strip_prefix('[')
            .unwrap()
            .strip_suffix(']')
            .unwrap();
        let xrefs: Vec<&str> = if inner.is_empty() {
            vec![]
        } else {
            inner.split(", ").collect()
        };
        assert_eq!(t["def"]["text"], text.as_str(), "{id} def");
        let got: Vec<&str> = t["def"]["xrefs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["id"].as_str().unwrap())
            .collect();
        assert_eq!(got, xrefs, "{id} def xrefs");
    }

    // comment: value, and the trailing {xref="..."} qualifier when present.
    if let Some(v) = raw_values(st, "comment").first() {
        let c = &t["comment"];
        match v.strip_suffix('}').and_then(|s| s.rsplit_once(" {")) {
            Some((text, quals)) => {
                assert_eq!(c["value"], text, "{id} comment");
                let (k, val) = quals.split_once('=').unwrap();
                assert_eq!(c["qualifiers"][0]["key"], k, "{id} comment qualifier");
                assert_eq!(c["qualifiers"][0]["value"], val.trim_matches('"'), "{id}");
            }
            None => {
                assert_eq!(c["value"], *v, "{id} comment");
                assert!(c.get("qualifiers").is_none(), "{id} comment");
            }
        }
    }

    // xrefs: ID and optional quoted description.
    for (i, v) in raw_values(st, "xref").iter().enumerate() {
        let x = &t["xrefs"][i];
        match v.split_once(' ') {
            Some((xid, desc)) => {
                assert_eq!(x["id"], xid, "{id} xref {i}");
                assert_eq!(
                    x["description"],
                    split_quoted(desc).0.as_str(),
                    "{id} xref {i}"
                );
            }
            None => {
                assert_eq!(x["id"], *v, "{id} xref {i}");
                assert!(x.get("description").is_none(), "{id} xref {i}");
            }
        }
    }

    // property_values: property, value, and datatype for quoted literals.
    for (i, v) in raw_values(st, "property_value").iter().enumerate() {
        let p = &t["property_values"][i];
        let (prop, rest) = v.split_once(' ').unwrap();
        assert_eq!(p["property"], prop, "{id} pv {i}");
        if rest.starts_with('"') {
            let (val, dt) = split_quoted(rest);
            assert_eq!(p["value"], val.as_str(), "{id} pv {i}");
            assert_eq!(p["datatype"], dt, "{id} pv {i}");
        } else {
            assert_eq!(p["value"], rest, "{id} pv {i}");
            assert!(p.get("datatype").is_none(), "{id} pv {i}");
        }
    }
}

#[test]
fn json_fields_match_source_clauses_for_sampled_terms() {
    let ids = sample_ids();
    assert!(ids.len() > 40);
    for id in &ids {
        let r = lib(id, true, 1);
        assert_eq!(r[0].id, *id, "exact-ID search did not return {id} first");
        let v = serde_json::to_value(&r[0]).unwrap();
        check_term_json(id, &v);
    }
}

#[test]
fn hp0000252_has_all_synonym_scopes_and_bare_local_id_does_not_match() {
    let t = &lib("HP:0000252", true, 1)[0];
    let v = serde_json::to_value(t).unwrap();
    // Expected scopes are read from the file rather than hard-coded.
    let raw_scopes: Vec<String> = raw_values(raw_stanza("HP:0000252"), "synonym")
        .iter()
        .map(|s| {
            split_quoted(s)
                .1
                .split_whitespace()
                .next()
                .unwrap()
                .to_string()
        })
        .collect();
    let got: Vec<&str> = v["synonyms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["scope"].as_str().unwrap())
        .collect();
    assert_eq!(got, raw_scopes);
    assert!(got.contains(&"BROAD") && got.contains(&"NARROW") && got.contains(&"EXACT"));
    // "Abnormally small head" and "Decreased size of head" are BROAD in hp.obo, and no
    // EXACT synonym or the name has all of their words: never matched. ("small
    // head" itself does match, via the EXACT synonym "Small head circumference".)
    for q in ["abnormally small head", "decreased size of head"] {
        assert!(
            !lib_ids(q, true, usize::MAX).contains(&"HP:0000252".to_string()),
            "{q}"
        );
    }
    assert!(lib_ids("small head", true, usize::MAX).contains(&"HP:0000252".to_string()));
    // "small calvarium" is only a NARROW synonym of HP:0000252: never matched.
    assert!(!lib_ids("small calvarium", true, usize::MAX).contains(&"HP:0000252".to_string()));
    // Bare local ID does not match by ID.
    assert!(!lib_ids("0000252", true, usize::MAX).contains(&"HP:0000252".to_string()));
}

// ---------------------------------------------------------------------------------------
// CLI tests
// ---------------------------------------------------------------------------------------

#[cfg(feature = "cli")]
mod cli {
    use super::*;

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

    fn stanzas(out: &str) -> Vec<&str> {
        if out.is_empty() {
            return vec![];
        }
        assert!(
            out.ends_with('\n') && !out.ends_with("\n\n"),
            "bad trailing newlines"
        );
        out.trim_end_matches('\n').split("\n\n").collect()
    }

    /// (flags, query, include_synonyms, limit) run through both the CLI and the library.
    const CASES: &[(&[&str], &str, bool, usize)] = &[
        (&[], "abnormal skull", true, 100),
        (&["--limit", "150"], "HP:00002", true, 150),
        (&["--limit", "40"], "HP:000027", true, 40),
        (&["--no-synonyms"], "microceph", false, 100),
        (&[], "small cranium", true, 100),
        (&[], "upper jaw", true, 100),
    ];

    fn args<'a>(flags: &[&'a str], json: bool, q: &'a str) -> Vec<&'a str> {
        let mut a = flags.to_vec();
        if json {
            a.push("--json");
        }
        a.push(HP);
        a.push(q);
        a
    }

    /// Every printed stanza parses with fastobo; its id/name match the JSON output for the
    /// same query; its clause tags are the source stanza's tags in file order; qualifiers
    /// and trailing comments are kept.
    #[test]
    fn text_stanzas_round_trip_and_match_json() {
        for (flags, q, _, _) in CASES {
            let text = obos(&args(flags, false, q));
            let json: Json = serde_json::from_str(&obos(&args(flags, true, q))).unwrap();
            let arr = json.as_array().unwrap();
            let st = stanzas(&text);
            assert!(!st.is_empty(), "{q}: no results");
            assert_eq!(
                st.len(),
                arr.len(),
                "{q}: text and JSON result counts differ"
            );
            for (i, s) in st.iter().enumerate() {
                let doc = fastobo::from_str(format!("format-version: 1.2\n\n{s}\n"))
                    .unwrap_or_else(|e| panic!("{q} stanza {i} does not parse: {e}\n{s}"));
                assert_eq!(doc.entities().len(), 1, "{q} stanza {i}");
                let frame = doc.entities()[0].as_term().expect("not a [Term] frame");
                let id = frame.id().as_inner().to_string();
                assert_eq!(arr[i]["id"], id.as_str(), "{q} stanza {i}");
                let name = frame.clauses().iter().find_map(|l| match l.as_inner() {
                    fastobo::ast::TermClause::Name(n) => Some(n.as_str().to_string()),
                    _ => None,
                });
                assert_eq!(arr[i]["name"], name.unwrap().as_str(), "{q} stanza {i}");

                let raw = raw_stanza(&id);
                let lines: Vec<&str> = s.lines().collect();
                assert_eq!(lines[0], "[Term]");
                let tags: Vec<&str> = lines[1..]
                    .iter()
                    .map(|l| l.split_once(": ").unwrap().0)
                    .collect();
                let raw_tags: Vec<&str> = raw.iter().map(|(t, _)| t.as_str()).collect();
                assert_eq!(
                    tags, raw_tags,
                    "{id}: clause tags/order differ from the file"
                );
                // is_a lines (ID plus "! comment") and comment qualifiers survive verbatim.
                for (line, (tag, val)) in lines[1..].iter().zip(raw) {
                    if tag == "is_a" || tag == "alt_id" {
                        assert_eq!(*line, format!("{tag}: {val}"), "{id}");
                    }
                    if tag == "comment" && val.ends_with('}') {
                        let qual = &val[val.rfind(" {").unwrap()..];
                        assert!(line.ends_with(qual), "{id}: qualifier lost: {line}");
                    }
                }
                check_term_json(&id, &arr[i]);
            }
        }
    }

    /// The CLI's JSON is the library's serialization of the same search, byte for byte.
    #[test]
    fn cli_json_equals_library_json() {
        for (flags, q, syn, limit) in CASES {
            let cli = obos(&args(flags, true, q));
            let lib_json = serde_json::to_string(&lib(q, *syn, *limit)).unwrap() + "\n";
            assert_eq!(cli, lib_json, "{flags:?} {q}");
        }
    }

    /// `--limit 5` prints the first 5 of the full ranked list; the default is 100.
    #[test]
    fn cli_limit_takes_top_of_ranking() {
        let ids_of = |out: &str| -> Vec<String> {
            let v: Json = serde_json::from_str(out).unwrap();
            v.as_array()
                .unwrap()
                .iter()
                .map(|t| t["id"].as_str().unwrap().to_string())
                .collect()
        };
        let all = ids_of(&obos(&["--json", "--limit", "1000000", HP, "abn"]));
        assert!(all.len() > 100);
        let five = obos(&["--limit", "5", HP, "abn"]);
        let five_ids: Vec<String> = stanzas(&five)
            .iter()
            .map(|s| {
                s.lines()
                    .nth(1)
                    .unwrap()
                    .strip_prefix("id: ")
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(five_ids, all[..5]);
        assert_eq!(ids_of(&obos(&["--json", HP, "abn"])), all[..100]);
        // The library agrees on the full list.
        assert_eq!(lib_ids("abn", true, usize::MAX), all);
    }

    /// The same file given relative to a different working directory, as an absolute path
    /// and as a file:// URL gives identical output.
    #[test]
    fn location_forms_give_identical_output() {
        let abs = std::fs::canonicalize("tests/fixtures/mini.obo").unwrap();
        let url = url::Url::from_file_path(&abs).unwrap().to_string();
        let run_in = |dir: &str, loc: &str| {
            let out = assert_cmd::cargo::cargo_bin_cmd!("obos")
                .current_dir(dir)
                .args(["search", "--json", loc, "microcephaly"])
                .assert()
                .success()
                .get_output()
                .stdout
                .clone();
            String::from_utf8(out).unwrap()
        };
        let base = run_in(".", "tests/fixtures/mini.obo");
        assert!(base.starts_with("[{\"id\":\"HP:0000252\""), "{base}");
        assert_eq!(run_in("tests/fixtures", "mini.obo"), base);
        assert_eq!(run_in("tests/fixtures", "./mini.obo"), base);
        assert_eq!(run_in("tests", "fixtures/mini.obo"), base);
        assert_eq!(run_in("/", abs.to_str().unwrap()), base);
        assert_eq!(run_in("/", &url), base);
    }
}

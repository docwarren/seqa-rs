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

use seqa_obos::{SearchOptions, SynonymScope, load, search, search_obo_text};

const MINI: &str = "./tests/fixtures/mini.obo";

async fn ids(q: &str) -> Vec<String> {
    load(MINI).await.unwrap();
    search(q, &SearchOptions::default())
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

#[tokio::test]
async fn microceph_excludes_obsolete() {
    let r = ids("microceph").await;
    for id in [
        "HP:0000252",
        "HP:0000253",
        "HP:0000254",
        "HP:0000255",
        "HP:0000256",
    ] {
        assert!(r.contains(&id.to_string()), "{id} missing from {r:?}");
    }
    assert!(!r.contains(&"HP:0000257".to_string()));
}

#[tokio::test]
async fn progressive_micr() {
    let r = ids("progressive micr").await;
    for id in ["HP:0000253", "HP:0000254", "HP:0000255"] {
        assert!(r.contains(&id.to_string()), "{id} missing from {r:?}");
    }
    assert!(!r.contains(&"HP:0000408".to_string()));
}

#[tokio::test]
async fn id_search_returns_full_entry() {
    load(MINI).await.unwrap();
    let r = search("hp:0000252", &SearchOptions::default()).unwrap();
    let t = &r[0];
    assert_eq!(t.id, "HP:0000252");
    assert_eq!(t.name, "Microcephaly");
    assert_eq!(t.alt_ids[0].value.id, "HP:0001366");
    let def = &t.def.as_ref().unwrap().value;
    assert!(def.text.starts_with("Head circumference below"));
    assert_eq!(def.xrefs.len(), 2);
    let scopes: Vec<SynonymScope> = t.synonyms.iter().map(|s| s.value.scope).collect();
    for s in [
        SynonymScope::Exact,
        SynonymScope::Narrow,
        SynonymScope::Broad,
        SynonymScope::Related,
    ] {
        assert!(scopes.contains(&s));
    }
    assert_eq!(t.synonyms[0].value.r#type.as_deref(), Some("layperson"));
    assert_eq!(t.synonyms[0].value.xrefs[0].id, "ORCID:0000-0001-5208-3432");
    assert_eq!(t.xrefs.len(), 2);
    assert_eq!(t.xrefs[0].qualifiers[0].key, "source");
    assert_eq!(t.is_a[0].value.id, "HP:0040195");
    assert_eq!(
        t.is_a[0].comment.as_deref(),
        Some("Decreased head circumference")
    );
}

#[tokio::test]
async fn obo_text_search() {
    load(MINI).await.unwrap();
    let text = search_obo_text("hp:0000252", &SearchOptions::default()).unwrap();
    assert!(text[0].starts_with("[Term]\nid: HP:0000252\nname: Microcephaly\n"));
}

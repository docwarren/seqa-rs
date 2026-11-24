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

//! Load -> replace -> search through the public API, failed loads, and no-op reloads.
//!
//! Every test here changes the process-wide ontology, so they are serialised with `LOCK`
//! and each one establishes the state it needs before asserting.

use std::path::{Path, PathBuf};
use tokio::sync::{Mutex, MutexGuard};

use seqa_obos::{Error, SearchOptions, load, search};

const MINI: &str = "./tests/fixtures/mini.obo";
const MINI2: &str = "tests/fixtures/mini2.obo";

static LOCK: Mutex<()> = Mutex::const_new(());

async fn serial() -> MutexGuard<'static, ()> {
    LOCK.lock().await
}

fn ids(q: &str) -> Vec<String> {
    search(q, &SearchOptions::default())
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

/// A fresh, empty directory under cargo's per-test-target temp dir.
fn tmp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("obos-lifecycle")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Asserts the active ontology is mini2: its terms are found and mini's are gone.
fn assert_mini2_active() {
    assert_eq!(ids("microcephaly"), ["MONDO:0001149"]);
    assert!(ids("HP:0000252").is_empty(), "mini term still searchable");
    assert!(ids("progressive micr").is_empty());
}

#[tokio::test]
async fn load_replace_then_search_sees_only_new_ontology() {
    let _g = serial().await;
    load(MINI).await.unwrap();
    let r = ids("microcephaly");
    assert_eq!(r[0], "HP:0000252");
    assert!(r.iter().all(|id| id.starts_with("HP:")), "{r:?}");

    load(MINI2).await.unwrap();
    assert_mini2_active();
    // Synonym matching works on the replacement too.
    assert_eq!(ids("small head"), ["MONDO:0001149"]);
    // ID-prefix matching on a non-HPO ontology (no prefix filtering).
    assert_eq!(ids("mondo:"), ["MONDO:0000001", "MONDO:0001149"]);

    // And back again: the first ontology is fetched afresh and fully replaces mini2.
    load(MINI).await.unwrap();
    assert_eq!(ids("hp:0000252")[0], "HP:0000252");
    assert!(ids("mondo:").is_empty());
}

#[tokio::test]
async fn failed_loads_leave_previous_ontology_searchable() {
    let _g = serial().await;
    let dir = tmp_dir("failed");
    let malformed = dir.join("malformed.obo");
    std::fs::write(&malformed, "format-version: 1.2\n\n[Term\nid: HP:1\n").unwrap();
    let malformed_url = url::Url::from_file_path(&malformed).unwrap().to_string();

    load(MINI).await.unwrap();
    load(MINI2).await.unwrap();
    assert_mini2_active();

    // Missing file: NotFound naming the normalised location.
    let missing = dir.join("missing.obo");
    match load(missing.to_str().unwrap()).await {
        Err(Error::NotFound(loc)) => {
            assert!(loc.contains("missing.obo"), "{loc}");
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert_mini2_active();

    // Malformed OBO: Parse error with the location and the parser's message.
    match load(malformed.to_str().unwrap()).await {
        Err(Error::Parse(msg)) => {
            let prefix = format!("{malformed_url}: ");
            assert!(msg.starts_with(&prefix), "{msg}");
            assert!(msg.len() > prefix.len(), "parser message missing: {msg}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
    assert_mini2_active();

    // Unsupported scheme.
    let err = load("ftp://host/hp.obo").await.unwrap_err();
    assert!(matches!(err, Error::InvalidLocation(_)), "{err:?}");
    assert_mini2_active();

    // A directory is not an OBO object: some error, never a panic or a replacement.
    let err = load(dir.to_str().unwrap()).await;
    assert!(err.is_err(), "loading a directory succeeded");
    assert_mini2_active();

    // A failed load must not poison the "same location" check: after failing on the
    // malformed file, fixing it and loading again must actually fetch and parse it.
    std::fs::write(
        &malformed,
        "format-version: 1.2\n\n[Term]\nid: FIX:0000001\nname: Fixed term\n",
    )
    .unwrap();
    load(malformed.to_str().unwrap()).await.unwrap();
    assert_eq!(ids("fixed term"), ["FIX:0000001"]);
    assert!(ids("microcephaly").is_empty());
}

#[tokio::test]
async fn ontology_without_terms_replaces_and_searches_empty() {
    let _g = serial().await;
    let dir = tmp_dir("typedef-only");
    let path = dir.join("typedefs.obo");
    std::fs::write(
        &path,
        "format-version: 1.2\n\n[Typedef]\nid: part_of\nname: part of\n",
    )
    .unwrap();
    load(MINI).await.unwrap();
    load(path.to_str().unwrap()).await.unwrap();
    // Loaded (not NotLoaded), but nothing is searchable: typedefs are never returned.
    assert!(
        search("part of", &SearchOptions::default())
            .unwrap()
            .is_empty()
    );
    assert!(ids("microcephaly").is_empty());
}

#[tokio::test]
async fn cache_persists_across_many_searches() {
    let _g = serial().await;
    let dir = tmp_dir("persist");
    let path = dir.join("mini.obo");
    std::fs::copy("tests/fixtures/mini.obo", &path).unwrap();
    load(MINI2).await.unwrap();
    load(path.to_str().unwrap()).await.unwrap();
    // The source disappears; searches keep using the in-memory ontology.
    std::fs::remove_file(&path).unwrap();
    let first = search("microceph", &SearchOptions::default()).unwrap();
    for _ in 0..50 {
        assert_eq!(
            search("microceph", &SearchOptions::default()).unwrap(),
            first
        );
    }
}

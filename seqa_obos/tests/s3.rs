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

//! S3 integration tests. Opt-in: `cargo test --features cli -- --ignored`.
//! Needs AWS credentials in the standard environment and `OBOS_TEST_S3_URL` set to the
//! `s3://` URL of an HPO `.obo` file.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use seqa_obos::{Error, SearchOptions, Term, load, search};

fn url() -> String {
    match std::env::var("OBOS_TEST_S3_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => panic!(
            "OBOS_TEST_S3_URL is not set. Set it to the s3:// URL of an HPO .obo file \
             (with AWS credentials available) to run the S3 tests."
        ),
    }
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

fn ensure_loaded() {
    static LOADED: OnceLock<()> = OnceLock::new();
    let url = url();
    LOADED.get_or_init(|| {
        let start = Instant::now();
        block_on(load(&url)).unwrap();
        println!("loaded from S3 in {:?}", start.elapsed());
    });
}

fn run(q: &str) -> Vec<Term> {
    ensure_loaded();
    search(q, &SearchOptions::default()).unwrap()
}

#[test]
#[ignore = "needs AWS credentials and OBOS_TEST_S3_URL"]
fn load_and_search_microceph() {
    assert!(run("microceph").iter().any(|t| t.id == "HP:0000252"));
}

#[test]
#[ignore = "needs AWS credentials and OBOS_TEST_S3_URL"]
fn id_search_returns_full_entry() {
    let r = run("HP:0000252");
    let t = &r[0];
    assert_eq!(t.id, "HP:0000252");
    assert_eq!(t.name, "Microcephaly");
    assert!(t.alt_ids.iter().any(|a| a.value.id == "HP:0001366"));
    assert!(t.def.is_some());
    assert!(!t.synonyms.is_empty());
    assert!(t.is_a.iter().any(|p| p.value.id == "HP:0040195"
        && p.comment.as_deref() == Some("Decreased head circumference")));
}

#[test]
#[ignore = "needs AWS credentials and OBOS_TEST_S3_URL"]
fn exact_synonym_search_by_default() {
    assert!(run("small cranium").iter().any(|t| t.id == "HP:0000252"));
}

#[test]
#[ignore = "needs AWS credentials and OBOS_TEST_S3_URL"]
fn missing_key_is_not_found() {
    let mut missing = url::Url::parse(&url()).unwrap();
    missing
        .path_segments_mut()
        .unwrap()
        .pop()
        .push("obos-test-missing-object.obo");
    match block_on(load(missing.as_str())) {
        Err(Error::NotFound(loc)) => assert_eq!(loc, missing.as_str()),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
#[ignore = "needs AWS credentials and OBOS_TEST_S3_URL"]
fn reloading_same_url_is_a_no_op() {
    ensure_loaded();
    let start = Instant::now();
    block_on(load(&url())).unwrap();
    let elapsed = start.elapsed();
    // A real fetch and parse of HPO from S3 takes far longer than this.
    assert!(
        elapsed < Duration::from_millis(100),
        "reload took {elapsed:?}"
    );
    assert!(!run("microceph").is_empty());
}

#[cfg(feature = "cli")]
#[test]
#[ignore = "needs AWS credentials and OBOS_TEST_S3_URL"]
fn cli_search() {
    assert_cmd::cargo::cargo_bin_cmd!("obos")
        .args(["search", &url(), "microceph"])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("[Term]\nid: HP:0000252\n"));
}

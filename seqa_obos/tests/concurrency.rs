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

//! Concurrent searches and loads against the process-wide ontology.
//!
//! Tests are serialised with `LOCK` because they all swap the global ontology.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

use seqa_obos::{SearchOptions, load, search};

const MINI: &str = "./tests/fixtures/mini.obo";
const MINI2: &str = "./tests/fixtures/mini2.obo";
const HP: &str = "./tests/fixtures/hp.obo";

static LOCK: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
}

fn ids(q: &str) -> Vec<String> {
    search(q, &SearchOptions::default())
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

/// Many threads search while the main thread keeps swapping between two ontologies.
/// Every result list must be exactly the result for one of the two ontologies, never a
/// mix, a partial list, an error or a panic.
#[test]
fn searches_during_swaps_see_one_whole_ontology() {
    let _g = serial();
    let rt = runtime();
    const Q: &str = "microcephaly";

    rt.block_on(load(MINI)).unwrap();
    let a = ids(Q);
    rt.block_on(load(MINI2)).unwrap();
    let b = ids(Q);
    assert!(
        a.len() > 1 && a.iter().all(|i| i.starts_with("HP:")),
        "{a:?}"
    );
    assert_eq!(b, ["MONDO:0001149"]);

    let stop = AtomicBool::new(false);
    let seen_a = AtomicUsize::new(0);
    let seen_b = AtomicUsize::new(0);
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for _ in 0..8 {
            handles.push(s.spawn(|| {
                let mut n = 0usize;
                while !stop.load(Ordering::Relaxed) || n == 0 {
                    let got = ids(Q);
                    if got == a {
                        seen_a.fetch_add(1, Ordering::Relaxed);
                    } else if got == b {
                        seen_b.fetch_add(1, Ordering::Relaxed);
                    } else {
                        panic!("mixed or partial result: {got:?}");
                    }
                    n += 1;
                }
                n
            }));
        }
        for i in 0..100 {
            let loc = if i % 2 == 0 { MINI } else { MINI2 };
            rt.block_on(load(loc)).unwrap();
        }
        stop.store(true, Ordering::Relaxed);
        for h in handles {
            assert!(h.join().expect("search thread panicked") > 0);
        }
    });
    // 100 swaps ending on MINI2 (i = 99).
    assert_eq!(ids(Q), b);
    assert!(seen_a.load(Ordering::Relaxed) + seen_b.load(Ordering::Relaxed) > 0);
}

/// Many tasks load the same location at once: all succeed and the result is consistent.
#[test]
fn concurrent_loads_of_same_location_all_succeed() {
    let _g = serial();
    let rt = runtime();
    rt.block_on(async {
        load(MINI2).await.unwrap();
        let tasks: Vec<_> = (0..32).map(|_| tokio::spawn(load(MINI))).collect();
        for t in tasks {
            t.await.unwrap().unwrap();
        }
    });
    assert_eq!(ids("hp:0000252")[0], "HP:0000252");
    assert!(ids("mondo:").is_empty());
}

/// Concurrent loads of two different locations: all succeed, and afterwards exactly one
/// of the two ontologies is active ("last to finish wins").
#[test]
fn concurrent_loads_of_different_locations_leave_one_ontology() {
    let _g = serial();
    let rt = runtime();
    rt.block_on(async {
        let tasks: Vec<_> = (0..32)
            .map(|i| tokio::spawn(load(if i % 2 == 0 { MINI } else { MINI2 })))
            .collect();
        for t in tasks {
            t.await.unwrap().unwrap();
        }
    });
    let hp = ids("hp:0000252");
    let mondo = ids("mondo:");
    assert!(
        (hp.first().map(String::as_str) == Some("HP:0000252") && mondo.is_empty())
            || (hp.is_empty() && mondo == ["MONDO:0000001", "MONDO:0001149"]),
        "inconsistent state: hp={hp:?} mondo={mondo:?}"
    );
}

/// While a slow load (hp.obo) is in flight, searches keep answering from the previous
/// ontology; once it completes they answer from the new one, and never go back.
#[test]
fn searches_during_slow_load_use_previous_ontology() {
    let _g = serial();
    let rt = runtime();
    rt.block_on(load(MINI)).unwrap();
    let old = ids("microcephaly");

    let handle = rt.spawn(load(HP));
    let mut switched: Option<Vec<String>> = None;
    let mut during = 0usize;
    while !handle.is_finished() {
        let got = search("microcephaly", &SearchOptions::default()).unwrap();
        let got_ids: Vec<String> = got.iter().map(|t| t.id.clone()).collect();
        if got_ids == old {
            assert!(
                switched.is_none(),
                "went back to the old ontology after the swap"
            );
            during += 1;
        } else {
            // Must be the whole new ontology's answer (checked against the final answer
            // below); HP:0005484 is in hp.obo but not in mini.obo.
            assert!(got_ids.contains(&"HP:0005484".to_string()), "{got_ids:?}");
            if let Some(prev) = &switched {
                assert_eq!(prev, &got_ids);
            }
            switched = Some(got_ids);
        }
    }
    rt.block_on(handle).unwrap().unwrap();
    let after = ids("microcephaly");
    assert_eq!(after[0], "HP:0000252");
    assert_ne!(after, old);
    if let Some(seen) = switched {
        assert_eq!(
            seen, after,
            "result seen mid-load differs from the final ontology"
        );
    }
    println!("{during} searches answered from the old ontology during the load");
}

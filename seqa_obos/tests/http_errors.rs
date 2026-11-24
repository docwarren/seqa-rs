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

//! Error paths over plain `http://` from a local test server, through the library and
//! the CLI. Offline: the server binds 127.0.0.1 on an ephemeral port.

use tokio::sync::{Mutex, MutexGuard};

use seqa_obos::{Error, SearchOptions, load, search};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

static LOCK: Mutex<()> = Mutex::const_new(());

async fn serial() -> MutexGuard<'static, ()> {
    LOCK.lock().await
}

const MALFORMED: &[u8] = b"format-version: 1.2\n\n[Term\nid: HP:1\nname: broken\n";

/// Routes (HEAD and GET):
/// - `/mini.obo`: tests/fixtures/mini.obo
/// - `/mini2.obo`: tests/fixtures/mini2.obo
/// - `/bad.obo`: 200 with malformed OBO
/// - `/forbidden.obo`: 403
/// - anything else: 404
async fn serve() -> String {
    let mini = std::fs::read("tests/fixtures/mini.obo").unwrap();
    let mini2 = std::fs::read("tests/fixtures/mini2.obo").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let (mini, mini2) = (mini.clone(), mini2.clone());
            tokio::spawn(async move {
                let mut req = Vec::new();
                let mut buf = [0u8; 1024];
                while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                    match sock.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => req.extend_from_slice(&buf[..n]),
                    }
                }
                let req = String::from_utf8_lossy(&req);
                let mut parts = req.split_whitespace();
                let (method, path) = (parts.next().unwrap(), parts.next().unwrap());
                let ok = |body: &[u8]| {
                    let mut r = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    if method == "GET" {
                        r.extend_from_slice(body);
                    }
                    r
                };
                let status = |line: &str| {
                    format!("HTTP/1.1 {line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                        .into_bytes()
                };
                let resp = match path {
                    "/mini.obo" => ok(&mini),
                    "/mini2.obo" => ok(&mini2),
                    "/bad.obo" => ok(MALFORMED),
                    "/forbidden.obo" => status("403 Forbidden"),
                    _ => status("404 Not Found"),
                };
                let _ = sock.write_all(&resp).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    format!("http://{addr}")
}

/// A URL on a port with nothing listening.
async fn refused_url() -> String {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    drop(l);
    format!("http://{addr}/x.obo")
}

fn ids(q: &str) -> Vec<String> {
    search(q, &SearchOptions::default())
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn library_http_failures_keep_previous_ontology() {
    let _g = serial().await;
    let base = serve().await;

    load(&format!("{base}/mini.obo")).await.unwrap();
    load(&format!("{base}/mini2.obo")).await.unwrap();
    assert_eq!(ids("microcephaly"), ["MONDO:0001149"]);

    let bad = format!("{base}/bad.obo");
    match load(&bad).await {
        Err(Error::Parse(msg)) => {
            assert!(msg.starts_with(&format!("{bad}: ")), "{msg}");
            assert!(msg.len() > bad.len() + 2, "parser message missing: {msg}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
    assert_eq!(ids("microcephaly"), ["MONDO:0001149"]);

    let forbidden = format!("{base}/forbidden.obo");
    match load(&forbidden).await {
        Err(Error::Store { .. }) => {}
        other => panic!("expected Store for 403, got {other:?}"),
    }
    assert_eq!(ids("microcephaly"), ["MONDO:0001149"]);

    let missing = format!("{base}/missing.obo");
    match load(&missing).await {
        Err(Error::NotFound(loc)) => assert_eq!(loc, missing),
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert_eq!(ids("microcephaly"), ["MONDO:0001149"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn library_unreachable_server_is_store_error() {
    let _g = serial().await;
    let base = serve().await;
    load(&format!("{base}/mini.obo")).await.unwrap();
    let url = refused_url().await;
    match load(&url).await {
        Err(Error::Store { .. }) => {}
        other => panic!("expected Store, got {other:?}"),
    }
    assert_eq!(ids("hp:0000252")[0], "HP:0000252");
}

#[cfg(feature = "cli")]
mod cli {
    use super::*;
    use assert_cmd::Command;
    use predicates::prelude::*;

    fn obos() -> Command {
        assert_cmd::cargo::cargo_bin_cmd!("obos")
    }

    /// Runs the CLI on a blocking thread so the server keeps running on the runtime.
    async fn run(args: Vec<String>) -> assert_cmd::assert::Assert {
        tokio::task::spawn_blocking(move || obos().arg("search").args(args).assert())
            .await
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_404_is_not_found_exit_1() {
        let base = serve().await;
        let url = format!("{base}/missing.obo");
        run(vec![url.clone(), "microceph".into()])
            .await
            .code(1)
            .stdout("")
            .stderr(predicate::str::contains("not found").and(predicate::str::contains(url)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_malformed_obo_is_parse_error_exit_1() {
        let base = serve().await;
        let url = format!("{base}/bad.obo");
        run(vec!["--json".into(), url.clone(), "microceph".into()])
            .await
            .code(1)
            .stdout("")
            .stderr(predicate::str::contains(format!(
                "failed to parse OBO from {url}"
            )));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_403_is_storage_error_exit_1() {
        let base = serve().await;
        run(vec![format!("{base}/forbidden.obo"), "microceph".into()])
            .await
            .code(1)
            .stdout("")
            .stderr(predicate::str::contains("storage error"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unreachable_server_is_storage_error_exit_1() {
        let url = refused_url().await;
        run(vec![url, "microceph".into()])
            .await
            .code(1)
            .stdout("")
            .stderr(predicate::str::contains("error:"));
    }

    /// Text and JSON over http:// are identical to the same file read from disk.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_output_matches_local_file_output() {
        let base = serve().await;
        let url = format!("{base}/mini.obo");
        for extra in [
            vec![],
            vec!["--json".to_string()],
            vec!["--no-synonyms".into()],
        ] {
            let mut via_http = extra.clone();
            via_http.extend([url.clone(), "microcephaly".into()]);
            let mut via_file = extra.clone();
            via_file.extend(["tests/fixtures/mini.obo".into(), "microcephaly".into()]);
            let h = run(via_http).await.success().get_output().stdout.clone();
            let f = run(via_file).await.success().get_output().stdout.clone();
            assert!(!h.is_empty());
            assert_eq!(
                String::from_utf8(h).unwrap(),
                String::from_utf8(f).unwrap(),
                "{extra:?}"
            );
        }
    }
}

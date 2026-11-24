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

//! Loading over plain `http://` from a local test server.

use seqa_obos::{Error, SearchOptions, load, search};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serves `tests/fixtures/mini.obo` at `/mini.obo` (HEAD and GET); 404 for anything else.
async fn serve() -> String {
    let body = std::fs::read("tests/fixtures/mini.obo").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let body = body.clone();
            tokio::spawn(async move {
                let mut req = Vec::new();
                let mut buf = [0u8; 1024];
                while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = sock.read(&mut buf).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    req.extend_from_slice(&buf[..n]);
                }
                let req = String::from_utf8_lossy(&req);
                let mut parts = req.split_whitespace();
                let (method, path) = (parts.next().unwrap(), parts.next().unwrap());
                let resp = if path == "/mini.obo" {
                    let mut r = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .into_bytes();
                    if method == "GET" {
                        r.extend_from_slice(&body);
                    }
                    r
                } else {
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_vec()
                };
                sock.write_all(&resp).await.unwrap();
                sock.shutdown().await.ok();
            });
        }
    });
    format!("http://{addr}")
}

#[tokio::test(flavor = "multi_thread")]
async fn http_load_and_missing_path() {
    let base = serve().await;

    let missing = format!("{base}/missing.obo");
    match load(&missing).await {
        Err(Error::NotFound(loc)) => assert_eq!(loc, missing),
        other => panic!("expected NotFound, got {other:?}"),
    }

    load(&format!("{base}/mini.obo")).await.unwrap();
    let r = search("microcephaly", &SearchOptions::default()).unwrap();
    assert_eq!(r[0].id, "HP:0000252");
}

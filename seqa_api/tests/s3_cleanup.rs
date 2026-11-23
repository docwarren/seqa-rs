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

mod common;

use common::s3_cleanup::DeleteOnDrop;
use seqa_core::stores::StoreService;

const S3_NOTES: &str = "s3://com.gmail.docarw/test_data/notes";

#[tokio::test]
async fn delete_on_drop_removes_the_object() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/api_delete_on_drop.txt");
    store.put_object(&path, b"temporary").await.unwrap();
    let guard = DeleteOnDrop::new(&path);
    drop(guard);
    assert!(store.get_object(&path).await.is_err());
}

#[tokio::test]
async fn delete_on_drop_runs_when_a_test_panics() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/api_delete_on_panic.txt");
    store.put_object(&path, b"temporary").await.unwrap();
    let guarded_path = path.clone();
    let failing_test = tokio::spawn(async move {
        let _guard = DeleteOnDrop::new(&guarded_path);
        panic!("assertion failed while the guard was held");
    });
    let outcome = failing_test.await;
    assert!(outcome.is_err());
    assert!(store.get_object(&path).await.is_err());
}

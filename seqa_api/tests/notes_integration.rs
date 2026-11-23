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

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use seqa_api::notes::notes_error::NotesError;
use seqa_api::notes::{SaveNoteRequest, read_note, save_note};
use seqa_core::stores::StoreService;
use std::sync::Arc;
use tokio::task::JoinSet;

mod common;

use common::s3_cleanup::DeleteOnDrop;

const S3_NOTES: &str = "s3://com.gmail.docarw/test_data/notes";
const MISSING_BUCKET: &str = "seqa-parity-no-such-bucket-3b9e1c";

fn every_byte() -> Vec<u8> {
    (0..=255).collect()
}

fn save_request(path: &str, bytes: &[u8]) -> SaveNoteRequest {
    SaveNoteRequest {
        path: path.to_string(),
        data: STANDARD.encode(bytes),
    }
}

struct TemporaryDirectory(String);

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temporary_notes_directory(name: &str) -> TemporaryDirectory {
    let directory = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    TemporaryDirectory(directory)
}

async fn assert_binary_round_trip(store: &StoreService, path: &str) {
    let bytes = every_byte();
    let request = save_request(path, &bytes);
    save_note(store, &request).await.unwrap();

    let stored = store.get_object(path).await.unwrap();
    assert_eq!(stored, bytes);

    let response = read_note(store, path).await.unwrap();
    let decoded = STANDARD.decode(&response.data).unwrap();
    assert_eq!(decoded, bytes);
    assert_eq!(response.data, request.data);
}

#[tokio::test]
async fn s3_binary_note_round_trip_is_byte_exact() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/core_every_byte.bin");
    let _cleanup = DeleteOnDrop::new(&path);
    assert_binary_round_trip(&store, &path).await;
}

#[tokio::test]
async fn local_binary_note_round_trip_is_byte_exact() {
    let store = StoreService::new();
    let directory = temporary_notes_directory("notes_binary_round_trip");
    let path = format!("file://{}/notes/every_byte.bin", directory.0);
    assert_binary_round_trip(&store, &path).await;
}

#[tokio::test]
async fn raw_object_reads_back_as_padded_base64() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/core_raw_put.bin");
    let _cleanup = DeleteOnDrop::new(&path);
    let bytes = [0xff_u8, 0x00, 0xfe, 0x7f];
    store.put_object(&path, &bytes).await.unwrap();
    let response = read_note(&store, &path).await.unwrap();
    assert_eq!(response.data, "/wD+fw==");
}

#[tokio::test]
async fn empty_note_round_trip() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/core_empty.txt");
    let _cleanup = DeleteOnDrop::new(&path);
    let request = save_request(&path, &[]);
    save_note(&store, &request).await.unwrap();
    let response = read_note(&store, &path).await.unwrap();
    assert_eq!(response.data, "");
}

#[tokio::test]
async fn note_with_space_in_name_round_trip() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/core note with spaces.txt");
    let _cleanup = DeleteOnDrop::new(&path);
    let request = save_request(&path, b"spaced");
    save_note(&store, &request).await.unwrap();
    let response = read_note(&store, &path).await.unwrap();
    assert_eq!(response.data, request.data);
}

#[tokio::test]
async fn unpadded_base64_is_rejected_and_nothing_written() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/core_never_written_unpadded.txt");
    let _cleanup = DeleteOnDrop::new(&path);
    let request = SaveNoteRequest {
        path: path.clone(),
        data: "SGVsbG8gd29ybGQ".to_string(),
    };
    let result = save_note(&store, &request).await;
    assert!(matches!(result, Err(NotesError::InvalidData { .. })), "{result:?}");
    assert!(store.get_object(&path).await.is_err());
}

#[tokio::test]
async fn url_safe_base64_is_rejected() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/core_never_written_url_safe.txt");
    let _cleanup = DeleteOnDrop::new(&path);
    let request = SaveNoteRequest {
        path: path.clone(),
        data: "_-8=".to_string(),
    };
    let result = save_note(&store, &request).await;
    assert!(matches!(result, Err(NotesError::InvalidData { .. })), "{result:?}");
    assert!(store.get_object(&path).await.is_err());
}

#[tokio::test]
async fn save_to_missing_bucket_is_a_store_error() {
    let store = StoreService::new();
    let path = format!("s3://{MISSING_BUCKET}/analysis/notes/today.txt");
    let request = save_request(&path, b"Hello world");
    let error = save_note(&store, &request).await.unwrap_err();
    assert!(matches!(error, NotesError::Store { .. }), "{error:?}");
    assert!(error.to_string().contains(&path), "{error}");
}

#[tokio::test]
async fn read_object_through_traversal_is_rejected_before_the_store() {
    let store = StoreService::new();
    let path = "s3://com.gmail.docarw/test_data/notes/../NA12877.bam";
    let result = read_note(&store, path).await;
    assert!(matches!(result, Err(NotesError::InvalidPath(_))), "{result:?}");
}

#[tokio::test]
async fn concurrent_saves_share_one_store() {
    let store = Arc::new(StoreService::new());
    let bytes = every_byte();
    let mut saves = JoinSet::new();
    let paths: Vec<String> = (0..8)
        .map(|number| format!("{S3_NOTES}/core_concurrent_{number}.bin"))
        .collect();
    let _cleanup: Vec<DeleteOnDrop> = paths.iter().map(|path| DeleteOnDrop::new(path)).collect();
    for path in &paths {
        let store = Arc::clone(&store);
        let request = save_request(path, &bytes);
        saves.spawn(async move { save_note(&store, &request).await });
    }
    let results = saves.join_all().await;
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    for path in &paths {
        let stored = store.get_object(path).await.unwrap();
        assert_eq!(stored, bytes, "{path}");
    }
}

#[tokio::test]
async fn overwrite_with_shorter_note_truncates() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/core_truncate.bin");
    let _cleanup = DeleteOnDrop::new(&path);
    let long = save_request(&path, &every_byte());
    save_note(&store, &long).await.unwrap();
    let short = save_request(&path, b"x");
    save_note(&store, &short).await.unwrap();
    let stored = store.get_object(&path).await.unwrap();
    assert_eq!(stored, b"x");
}

#[tokio::test]
async fn local_overwrite_with_shorter_note_truncates() {
    let store = StoreService::new();
    let directory = temporary_notes_directory("notes_truncate");
    let path = format!("file://{}/notes/truncate.bin", directory.0);
    let long = save_request(&path, &every_byte());
    save_note(&store, &long).await.unwrap();
    let short = save_request(&path, b"x");
    save_note(&store, &short).await.unwrap();
    let stored = store.get_object(&path).await.unwrap();
    assert_eq!(stored, b"x");
}

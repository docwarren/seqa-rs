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

use seqa_api::notes::notes_error::NotesError;
use seqa_api::notes::{
    ReadNoteRequest, ReadNoteResponse, SaveNoteRequest, read_note, save_note, validate_note_path,
};
use seqa_core::stores::StoreService;

mod common;

use common::s3_cleanup::DeleteOnDrop;

const S3_NOTES: &str = "s3://com.gmail.docarw/test_data/notes";
const HELLO_WORLD_BASE64: &str = "SGVsbG8gd29ybGQ=";

fn assert_invalid_path(path: &str) {
    let result = validate_note_path(path);
    assert!(matches!(result, Err(NotesError::InvalidPath(_))), "{path} was not rejected");
}

#[test]
fn accepts_cloud_path_inside_notes_directory() {
    assert!(validate_note_path("s3://bucket/analysis/notes/today.txt").is_ok());
    assert!(validate_note_path("gs://bucket/analysis/notes/today.txt").is_ok());
    assert!(validate_note_path("az://account/container/notes/today.txt").is_ok());
}

#[test]
fn accepts_azure_path_with_notes_below_container() {
    assert!(validate_note_path("az://account/notes/analysis/notes/today.txt").is_ok());
}

#[test]
fn rejects_azure_container_named_notes() {
    assert_invalid_path("az://account/notes/today.txt");
}

#[test]
fn accepts_local_path_inside_notes_directory() {
    assert!(validate_note_path("file:///home/user/analysis/notes/today.txt").is_ok());
}

#[test]
fn accepts_nested_directory_under_notes() {
    assert!(validate_note_path("s3://bucket/analysis/notes/images/plot.png").is_ok());
}

#[test]
fn rejects_http_path() {
    assert_invalid_path("https://example.com/analysis/notes/today.txt");
}

#[test]
fn rejects_path_outside_notes_directory() {
    assert_invalid_path("s3://bucket/analysis/sample.vcf.gz");
}

#[test]
fn rejects_similarly_named_directory() {
    assert_invalid_path("s3://bucket/analysis/mynotes/today.txt");
}

#[test]
fn rejects_traversal_out_of_notes_directory() {
    assert_invalid_path("s3://bucket/analysis/notes/../sample.vcf.gz");
}

#[test]
fn rejects_encoded_traversal_out_of_notes_directory() {
    assert_invalid_path("s3://bucket/analysis/notes/%2e%2e/sample.vcf.gz");
}

#[test]
fn rejects_notes_directory_itself() {
    assert_invalid_path("s3://bucket/analysis/notes/");
    assert_invalid_path("s3://bucket/analysis/notes");
}

#[test]
fn rejects_bucket_named_notes() {
    assert_invalid_path("s3://notes/today.txt");
}

#[test]
fn rejects_scheme_less_path() {
    assert_invalid_path("/analysis/notes/today.txt");
}

#[test]
fn deserializes_read_request() {
    let request: ReadNoteRequest = serde_json::from_str(r#"{"path": "s3://bucket/notes/a.txt"}"#).unwrap();
    assert_eq!(request.path, "s3://bucket/notes/a.txt");
}

#[test]
fn deserializes_save_request() {
    let request: SaveNoteRequest =
        serde_json::from_str(r#"{"path": "s3://bucket/notes/a.txt", "data": "SGVsbG8gd29ybGQ="}"#).unwrap();
    assert_eq!(request.path, "s3://bucket/notes/a.txt");
    assert_eq!(request.data, "SGVsbG8gd29ybGQ=");
}

#[test]
fn save_request_requires_data() {
    let result = serde_json::from_str::<SaveNoteRequest>(r#"{"path": "s3://bucket/notes/a.txt"}"#);
    assert!(result.is_err());
}

#[test]
fn serializes_read_response() {
    let response = ReadNoteResponse {
        data: "SGVsbG8gd29ybGQ=".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert_eq!(json, r#"{"data":"SGVsbG8gd29ybGQ="}"#);
}

fn save_request(path: &str, data: &str) -> SaveNoteRequest {
    SaveNoteRequest {
        path: path.to_string(),
        data: data.to_string(),
    }
}

async fn assert_round_trip(store: &StoreService, path: &str) {
    let request = save_request(path, HELLO_WORLD_BASE64);
    save_note(store, &request).await.unwrap();
    let response = read_note(store, path).await.unwrap();
    assert_eq!(response.data, HELLO_WORLD_BASE64);
}

#[tokio::test]
async fn s3_note_round_trip() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/round_trip.txt");
    let _cleanup = DeleteOnDrop::new(&path);
    assert_round_trip(&store, &path).await;
    let bytes = store.get_object(&path).await.unwrap();
    assert_eq!(bytes, b"Hello world");
}

#[tokio::test]
async fn s3_note_overwrite() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/overwrite.txt");
    let _cleanup = DeleteOnDrop::new(&path);
    let first = save_request(&path, "Zmlyc3Q=");
    save_note(&store, &first).await.unwrap();
    let second = save_request(&path, "c2Vjb25k");
    save_note(&store, &second).await.unwrap();
    let response = read_note(&store, &path).await.unwrap();
    assert_eq!(response.data, "c2Vjb25k");
}

#[tokio::test]
async fn missing_note_is_not_found() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/does_not_exist.txt");
    let result = read_note(&store, &path).await;
    assert!(matches!(result, Err(NotesError::NotFound(_))), "{result:?}");
}

#[tokio::test]
async fn invalid_base64_is_rejected_and_nothing_written() {
    let store = StoreService::new();
    let path = format!("{S3_NOTES}/never_written_bad_data.txt");
    let _cleanup = DeleteOnDrop::new(&path);
    let request = save_request(&path, "not base64!");
    let result = save_note(&store, &request).await;
    assert!(matches!(result, Err(NotesError::InvalidData { .. })), "{result:?}");
    assert!(store.get_object(&path).await.is_err());
}

#[tokio::test]
async fn invalid_path_is_rejected_on_save_and_nothing_written() {
    let store = StoreService::new();
    let path = "s3://com.gmail.docarw/test_data/never_written_outside_notes.txt";
    let _cleanup = DeleteOnDrop::new(path);
    let request = save_request(path, HELLO_WORLD_BASE64);
    let result = save_note(&store, &request).await;
    assert!(matches!(result, Err(NotesError::InvalidPath(_))), "{result:?}");
    assert!(store.get_object(path).await.is_err());
}

#[tokio::test]
async fn invalid_path_is_rejected_on_read() {
    let store = StoreService::new();
    let result = read_note(&store, "s3://com.gmail.docarw/test.txt").await;
    assert!(matches!(result, Err(NotesError::InvalidPath(_))), "{result:?}");
}

struct TemporaryDirectory(String);

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn local_note_round_trip() {
    let store = StoreService::new();
    let round_trip_directory = format!("{}/tests/data/notes_round_trip", env!("CARGO_MANIFEST_DIR"));
    let _cleanup = TemporaryDirectory(round_trip_directory.clone());
    let path = format!("file://{round_trip_directory}/notes/local_round_trip.txt");
    assert_round_trip(&store, &path).await;
}

#[tokio::test]
async fn store_failure_names_the_path() {
    let store = StoreService::new();
    let path = format!(
        "file://{}/tests/data/test.txt/notes/cannot_write.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let request = save_request(&path, HELLO_WORLD_BASE64);
    let error = save_note(&store, &request).await.unwrap_err();
    assert!(matches!(error, NotesError::Store { .. }), "{error:?}");
    assert!(error.to_string().contains(&path), "{error}");
}

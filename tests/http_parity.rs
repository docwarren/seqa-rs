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

// Shared HTTP parity table for `seqa_axum` and `seqa_lambda`.
//
// Both servers are binary crates, so no single test can call both. Each
// server's inline test module includes this file with `include!`, sends every
// [`ParityCase`] through its own handler, and checks the result with
// [`check_case`]. Passing on both servers means both give the same status and
// the same body for every request in the table.

use seqa_api::local_uri::LocalUriError;
use seqa_api::notes::notes_error::NotesError;
use seqa_api::notes::{read_note, save_note, SaveNoteRequest};
use seqa_api::search_request::SearchRequest;
use seqa_core::reference_data;
use seqa_core::stores::buckets::CloudBackend;
use seqa_core::stores::StoreService;
use seqa_core::utils::supported_formats;
use serde_json::{json, Value};

mod s3_cleanup {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../seqa_api/tests/common/s3_cleanup.rs"));
}

pub use s3_cleanup::DeleteOnDrop;

pub const S3_NOTES: &str = "s3://com.gmail.docarw/test_data/notes";
pub const VCF: &str = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz";
pub const VCF_INDEX: &str = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi";
pub const MISSING_VCF: &str = "s3://com.gmail.docarw/test_data/parity_no_such_file.vcf.gz";
pub const MISSING_VCF_INDEX: &str = "s3://com.gmail.docarw/test_data/parity_no_such_file.vcf.gz.tbi";
pub const MISSING_BUCKET: &str = "seqa-parity-no-such-bucket-3b9e1c";
pub const JSON_CONTENT_TYPE: &str = "application/json";
pub const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";
const OUTSIDE_NOTES_PATH: &str = "s3://com.gmail.docarw/test_data/parity_never_written.txt";

pub const EVERY_BYTE_BASE64: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w==";

/// A request both servers receive unchanged.
pub struct ParityRequest {
    pub method: &'static str,
    pub path: String,
    pub content_type: Option<&'static str>,
    pub body: String,
}

/// What a server returned, reduced to the parts the table compares.
pub struct ObservedResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

/// The expected search result shape: coordinates in order, the first line of
/// each coordinate's results (`None` for no results), and a header line that
/// must be present (`None` for an empty header).
pub struct ExpectedSearch {
    pub coordinates: Vec<&'static str>,
    pub first_line_prefixes: Vec<Option<&'static str>>,
    pub header_line_prefix: Option<&'static str>,
}

/// The body a case expects.
pub enum ExpectedBody {
    Json(Value),
    ErrorMessage(String),
    ErrorMessageContaining(String),
    AnyErrorMessage,
    ArrayContaining { field: &'static str, value: String },
    Search(ExpectedSearch),
    Empty,
}

/// One row of the parity table.
pub struct ParityCase {
    pub name: &'static str,
    pub request: ParityRequest,
    pub status: u16,
    pub expected: ExpectedBody,
}

/// Index files the table's searches download, so callers can hold an
/// `IndexGuard` for each.
pub fn indexes_used() -> Vec<&'static str> {
    vec![VCF_INDEX, MISSING_VCF_INDEX]
}

/// The URI of `file_name` in the S3 notes directory the table uses.
pub fn note_path(file_name: &str) -> String {
    format!("{S3_NOTES}/{file_name}")
}

fn bad_data_note_path() -> String {
    note_path("parity_never_written_bad_data.txt")
}

fn form_note_path() -> String {
    note_path("parity_never_written_form.txt")
}

fn no_data_note_path() -> String {
    note_path("parity_never_written_no_data.txt")
}

fn rejected_save_paths() -> Vec<String> {
    vec![
        bad_data_note_path(),
        form_note_path(),
        no_data_note_path(),
        OUTSIDE_NOTES_PATH.to_string(),
    ]
}

/// Guards for every object the table's rejected saves target, so a save that
/// is wrongly accepted does not leave the object behind.
pub fn rejected_save_guards() -> Vec<DeleteOnDrop> {
    rejected_save_paths()
        .iter()
        .map(|path| DeleteOnDrop::new(path))
        .collect()
}

/// Describes every object that the table's rejected saves wrote.
pub async fn rejected_saves_written() -> Vec<String> {
    let mut written = Vec::new();
    for path in rejected_save_paths() {
        if let Err(failure) = check_not_written(&path).await {
            written.push(failure);
        }
    }
    written
}

/// All 256 byte values in order; the decoded form of [`EVERY_BYTE_BASE64`].
pub fn every_byte() -> Vec<u8> {
    (0..=255).collect()
}

fn request(method: &'static str, path: &str, content_type: Option<&'static str>, body: String) -> ParityRequest {
    ParityRequest {
        method,
        path: path.to_string(),
        content_type,
        body,
    }
}

fn get(path: &str) -> ParityRequest {
    request("GET", path, None, String::new())
}

fn post_json(path: &str, body: Value) -> ParityRequest {
    let text = body.to_string();
    request("POST", path, Some(JSON_CONTENT_TYPE), text)
}

fn post_text(path: &str, body: &str) -> ParityRequest {
    request("POST", path, Some(JSON_CONTENT_TYPE), body.to_string())
}

/// `POST /notes/read` for `path`.
pub fn read_note_request(path: &str) -> ParityRequest {
    let body = json!({ "path": path });
    post_json("/notes/read", body)
}

/// `POST /notes/save` writing `data` (base64) to `path`.
pub fn save_note_request(path: &str, data: &str) -> ParityRequest {
    let body = json!({ "path": path, "data": data });
    post_json("/notes/save", body)
}

/// `POST /notes/save` sent as an HTML form would send it, which browsers do
/// cross-origin without a CORS preflight.
pub fn form_save_note_request(path: &str) -> ParityRequest {
    let body = format!("path={path}&data=SGVsbG8gd29ybGQ%3D");
    request("POST", "/notes/save", Some(FORM_CONTENT_TYPE), body)
}

fn case(name: &'static str, request: ParityRequest, status: u16, expected: ExpectedBody) -> ParityCase {
    ParityCase {
        name,
        request,
        status,
        expected,
    }
}

fn not_found(method: &'static str, path: &str) -> ParityCase {
    let message = format!("Not found: {method} {path}");
    let request = request(method, path, None, String::new());
    ParityCase {
        name: "not found",
        request,
        status: 404,
        expected: ExpectedBody::ErrorMessage(message),
    }
}

fn json_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}

fn local_uri_message(uri: &str) -> String {
    let error = LocalUriError(uri.to_string());
    error.to_string()
}

fn invalid_note_path_message(path: &str) -> String {
    let error = NotesError::InvalidPath(path.to_string());
    error.to_string()
}

fn note_not_found_message(path: &str) -> String {
    let error = NotesError::NotFound(path.to_string());
    error.to_string()
}

fn unknown_backend_message(backend: &str) -> String {
    let error = CloudBackend::from_id(backend).unwrap_err();
    error.to_string()
}

fn search_request_error_message(body: &Value) -> String {
    let search_request: SearchRequest = serde_json::from_value(body.clone()).unwrap();
    let error = search_request.to_search_options().unwrap_err();
    error.to_string()
}

fn search_rejected(name: &'static str, body: Value) -> ParityCase {
    let message = search_request_error_message(&body);
    let request = post_json("/search", body);
    case(name, request, 400, ExpectedBody::ErrorMessage(message))
}

fn local_listing_rejected(name: &'static str, uri: &str) -> ParityCase {
    let message = local_uri_message(uri);
    let body = json!({ "uri": uri });
    let request = post_json("/files/list", body);
    case(name, request, 400, ExpectedBody::ErrorMessage(message))
}

fn local_note_read_rejected(name: &'static str, path: &str) -> ParityCase {
    let message = local_uri_message(path);
    let request = read_note_request(path);
    case(name, request, 400, ExpectedBody::ErrorMessage(message))
}

fn invalid_note_read(name: &'static str, path: &str) -> ParityCase {
    let message = invalid_note_path_message(path);
    let request = read_note_request(path);
    case(name, request, 400, ExpectedBody::ErrorMessage(message))
}

fn backend_rejected(name: &'static str, backend: &str) -> ParityCase {
    let message = unknown_backend_message(backend);
    let path = format!("/files/buckets/{backend}");
    let request = get(&path);
    case(name, request, 400, ExpectedBody::ErrorMessage(message))
}

fn routing_cases() -> Vec<ParityCase> {
    vec![
        not_found("POST", "/anything"),
        not_found("GET", "/"),
        not_found("POST", "/"),
        not_found("POST", "/files"),
        not_found("GET", "/genes/cytobands/grch38/chr1"),
        not_found("GET", "/search"),
        not_found("GET", "/files/list"),
        not_found("GET", "/notes/read"),
        not_found("GET", "/notes/save"),
        not_found("POST", "/files/formats"),
        not_found("POST", "/cytobands/grch38/chr1"),
        not_found("PUT", "/notes/save"),
        not_found("DELETE", "/notes/read"),
        not_found("PATCH", "/search"),
        not_found("GET", "/genes/symbols/grch38"),
        not_found("GET", "/cytobands/grch38/chr1/extra"),
        case("unknown path with trailing slash", get("/anything/"), 404, ExpectedBody::AnyErrorMessage),
        case("bucket listing with no backend", get("/files/buckets/"), 404, ExpectedBody::AnyErrorMessage),
    ]
}

fn head_formats_request() -> ParityRequest {
    request("HEAD", "/files/formats", None, String::new())
}

fn head_answered_case() -> ParityCase {
    let request = head_formats_request();
    case("HEAD is answered", request, 200, ExpectedBody::Empty)
}

fn head_not_served_case() -> ParityCase {
    let message = "Not found: HEAD /files/formats".to_string();
    let request = head_formats_request();
    case("HEAD is not served", request, 404, ExpectedBody::ErrorMessage(message))
}

/// `HEAD /files/formats`. Answering `HEAD` on GET routes is optional: a server
/// that answers it returns 200 with an empty body, and one that does not
/// returns the JSON 404.
pub fn head_case(server_answers_head: bool) -> ParityCase {
    if server_answers_head {
        return head_answered_case();
    }
    head_not_served_case()
}

fn search_cases() -> Vec<ParityCase> {
    let two_coordinates_with_header = json!({
        "path": VCF,
        "coordinates": ["chr1:116549-116549", "chr1:1-2"],
        "with_header": true,
        "genome": "grch38",
    });
    let two_coordinates_without_header = json!({
        "path": VCF,
        "coordinates": ["chr1:116549-116549", "chr1:1-2"],
    });
    let expected_with_header = ExpectedSearch {
        coordinates: vec!["chr1:116549", "chr1:1-2"],
        first_line_prefixes: vec![Some("chr1\t116549"), None],
        header_line_prefix: Some("#CHROM"),
    };
    let expected_without_header = ExpectedSearch {
        coordinates: vec!["chr1:116549", "chr1:1-2"],
        first_line_prefixes: vec![Some("chr1\t116549"), None],
        header_line_prefix: None,
    };
    let charset_content_type = request(
        "POST",
        "/search",
        Some("application/json; charset=utf-8"),
        json!({ "path": VCF, "coordinates": [] }).to_string(),
    );
    let no_content_type = request(
        "POST",
        "/search",
        None,
        json!({ "path": VCF, "coordinates": ["chr1:1-2"] }).to_string(),
    );
    let missing_file = json!({ "path": MISSING_VCF, "coordinates": ["chr1:1-2"] });

    vec![
        case("search malformed body", post_text("/search", "not json"), 400, ExpectedBody::AnyErrorMessage),
        case(
            "search coordinates must be a list",
            post_json("/search", json!({ "path": VCF, "coordinates": "chr1:1-1000" })),
            400,
            ExpectedBody::AnyErrorMessage,
        ),
        case(
            "search missing path",
            post_json("/search", json!({ "coordinates": ["chr1:1-2"] })),
            400,
            ExpectedBody::AnyErrorMessage,
        ),
        case("search without content type", no_content_type, 400, ExpectedBody::AnyErrorMessage),
        case(
            "search content type with charset is parsed",
            charset_content_type,
            400,
            ExpectedBody::ErrorMessage("No coordinates".to_string()),
        ),
        search_rejected("search empty coordinates", json!({ "path": VCF, "coordinates": [] })),
        search_rejected(
            "search unparseable coordinate",
            json!({ "path": VCF, "coordinates": ["chr1:abc-def"] }),
        ),
        search_rejected(
            "search unsupported extension",
            json!({ "path": "s3://bucket/file.xyz", "coordinates": ["chr1:1-10"] }),
        ),
        search_rejected(
            "search unknown output format",
            json!({ "path": VCF, "coordinates": ["chr1:1-10"], "output_format": "nonsense" }),
        ),
        case(
            "search two coordinates with header, genome ignored",
            post_json("/search", two_coordinates_with_header),
            200,
            ExpectedBody::Search(expected_with_header),
        ),
        case(
            "search two coordinates without header",
            post_json("/search", two_coordinates_without_header),
            200,
            ExpectedBody::Search(expected_without_header),
        ),
        case(
            "search failure after validation",
            post_json("/search", missing_file),
            500,
            ExpectedBody::ErrorMessageContaining("Search Failed".to_string()),
        ),
    ]
}

fn listing_cases() -> Vec<ParityCase> {
    let missing_bucket_uri = format!("s3://{MISSING_BUCKET}/analysis/");
    let missing_bucket_message = format!("Error listing {missing_bucket_uri}");
    vec![
        local_listing_rejected("list file URI", "file:///etc"),
        local_listing_rejected("list file URI hidden by a tab", "fi\tle:///etc"),
        local_listing_rejected("list file URI hidden by a newline", "file\n:///etc"),
        local_listing_rejected("list file URI with leading space", " file:///etc"),
        local_listing_rejected("list upper-case file URI", "FILE:///etc"),
        local_listing_rejected("list file URI without slashes", "file:/etc"),
        local_listing_rejected("list scheme-less absolute path", "/etc/passwd"),
        local_listing_rejected("list scheme-less relative path", "etc/passwd"),
        local_listing_rejected("list empty URI", ""),
        case(
            "list missing uri field",
            post_json("/files/list", json!({ "wrong": "field" })),
            400,
            ExpectedBody::AnyErrorMessage,
        ),
        case(
            "list s3 prefix",
            post_json("/files/list", json!({ "uri": "s3://com.gmail.docarw/test_data/" })),
            200,
            ExpectedBody::ArrayContaining {
                field: "uri",
                value: "s3://com.gmail.docarw/test_data/NA12877.bam".to_string(),
            },
        ),
        case(
            "list store failure",
            post_json("/files/list", json!({ "uri": missing_bucket_uri })),
            500,
            ExpectedBody::ErrorMessageContaining(missing_bucket_message),
        ),
    ]
}

fn reference_cases() -> Vec<ParityCase> {
    let formats = json_value(&supported_formats());
    let genes = json_value(&reference_data::gene_coordinates("grch38", "refseq"));
    let hg38_genes = json_value(&reference_data::gene_coordinates("hg38", "refseq"));
    let chr1_bands = json_value(&reference_data::cytobands("grch38", "chr1"));
    let hg38_chr1_bands = json_value(&reference_data::cytobands("hg38", "chr1"));
    vec![
        backend_rejected("buckets unknown backend", "nonsense"),
        backend_rejected("buckets local backend", "local"),
        backend_rejected("buckets upper-case local backend", "LOCAL"),
        case("formats", get("/files/formats"), 200, ExpectedBody::Json(formats.clone())),
        case("formats with trailing slash", get("/files/formats/"), 200, ExpectedBody::Json(formats)),
        case("gene symbols", get("/genes/symbols/grch38/refseq"), 200, ExpectedBody::Json(genes.clone())),
        case(
            "gene symbols with trailing slash",
            get("/genes/symbols/grch38/refseq/"),
            200,
            ExpectedBody::Json(genes.clone()),
        ),
        case(
            "gene symbols unknown source falls back to RefSeq",
            get("/genes/symbols/grch38/nonsense"),
            200,
            ExpectedBody::Json(genes),
        ),
        case("cytobands", get("/cytobands/grch38/chr1"), 200, ExpectedBody::Json(chr1_bands.clone())),
        case(
            "cytobands with trailing slash",
            get("/cytobands/grch38/chr1/"),
            200,
            ExpectedBody::Json(chr1_bands.clone()),
        ),
        case(
            "cytobands with query string",
            get("/cytobands/grch38/chr1?cache=false"),
            200,
            ExpectedBody::Json(chr1_bands),
        ),
        case(
            "cytobands unknown genome falls back to hg38",
            get("/cytobands/nonsense/chr1"),
            200,
            ExpectedBody::Json(hg38_chr1_bands),
        ),
        case(
            "gene symbols unknown genome falls back to hg38",
            get("/genes/symbols/nonsense/refseq"),
            200,
            ExpectedBody::Json(hg38_genes),
        ),
        case(
            "cytobands unknown chromosome",
            get("/cytobands/grch38/chrNotReal"),
            200,
            ExpectedBody::Json(json!([])),
        ),
    ]
}

fn notes_cases() -> Vec<ParityCase> {
    let missing_note = note_path("parity_does_not_exist.txt");
    let missing_note_message = note_not_found_message(&missing_note);
    let bad_data_path = bad_data_note_path();
    let bad_data_message = format!("Invalid note data for {bad_data_path}");
    let missing_bucket_note = format!("s3://{MISSING_BUCKET}/analysis/notes/today.txt");
    let form_path = form_note_path();
    let form_read_body = format!("path={missing_note}");
    let save_scheme_less_message = local_uri_message("/analysis/notes/today.txt");
    let save_missing_data = json!({ "path": no_data_note_path() });
    let outside_notes_message = invalid_note_path_message(OUTSIDE_NOTES_PATH);

    vec![
        local_note_read_rejected("read file note", "file:///home/user/analysis/notes/today.txt"),
        local_note_read_rejected("read file note hidden by a tab", "fi\tle:///home/user/analysis/notes/today.txt"),
        local_note_read_rejected("read scheme-less note", "/analysis/notes/today.txt"),
        case(
            "save scheme-less note",
            save_note_request("/analysis/notes/today.txt", "SGVsbG8gd29ybGQ="),
            400,
            ExpectedBody::ErrorMessage(save_scheme_less_message),
        ),
        invalid_note_read("read outside notes directory", "s3://com.gmail.docarw/test_data/NA12877.bam"),
        invalid_note_read(
            "read traversal out of notes directory",
            "s3://com.gmail.docarw/test_data/notes/../NA12877.bam",
        ),
        invalid_note_read(
            "read encoded traversal out of notes directory",
            "s3://com.gmail.docarw/test_data/notes/%2e%2e/NA12877.bam",
        ),
        invalid_note_read("read https note", "https://example.com/analysis/notes/today.txt"),
        invalid_note_read("read notes directory itself", "s3://com.gmail.docarw/test_data/notes/"),
        invalid_note_read("read similarly named directory", "s3://com.gmail.docarw/test_data/mynotes/today.txt"),
        case(
            "read missing note",
            read_note_request(&missing_note),
            404,
            ExpectedBody::ErrorMessage(missing_note_message),
        ),
        case(
            "save invalid base64",
            save_note_request(&bad_data_path, "not base64!"),
            400,
            ExpectedBody::ErrorMessageContaining(bad_data_message),
        ),
        case(
            "save outside notes directory",
            save_note_request(OUTSIDE_NOTES_PATH, "SGVsbG8gd29ybGQ="),
            400,
            ExpectedBody::ErrorMessage(outside_notes_message),
        ),
        case("read not JSON", post_text("/notes/read", "not json"), 400, ExpectedBody::AnyErrorMessage),
        case(
            "read missing path field",
            post_json("/notes/read", json!({ "wrong": "field" })),
            400,
            ExpectedBody::AnyErrorMessage,
        ),
        case("save not JSON", post_text("/notes/save", "not json"), 400, ExpectedBody::AnyErrorMessage),
        case(
            "save missing data field",
            post_json("/notes/save", save_missing_data),
            400,
            ExpectedBody::AnyErrorMessage,
        ),
        case(
            "save form-encoded body is not JSON",
            form_save_note_request(&form_path),
            400,
            ExpectedBody::AnyErrorMessage,
        ),
        case(
            "read form-encoded body is not JSON",
            request("POST", "/notes/read", Some(FORM_CONTENT_TYPE), form_read_body),
            400,
            ExpectedBody::AnyErrorMessage,
        ),
        case(
            "save store failure",
            save_note_request(&missing_bucket_note, "SGVsbG8gd29ybGQ="),
            500,
            ExpectedBody::ErrorMessageContaining(missing_bucket_note.clone()),
        ),
    ]
}

/// The full parity table: every route in the `http-api` spec, its error
/// paths, and the local-URI bypass attempts.
pub fn parity_cases() -> Vec<ParityCase> {
    let mut cases = routing_cases();
    cases.extend(search_cases());
    cases.extend(listing_cases());
    cases.extend(reference_cases());
    cases.extend(notes_cases());
    cases
}

fn describe(case: &ParityCase) -> String {
    format!("[{}] {} {}", case.name, case.request.method, case.request.path)
}

fn parse_json(body: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(body).map_err(|error| {
        let text = String::from_utf8_lossy(body);
        format!("body is not JSON ({error}): {text}")
    })
}

fn check_json_content_type(response: &ObservedResponse) -> Result<(), String> {
    match response.content_type.as_deref() {
        Some(JSON_CONTENT_TYPE) => Ok(()),
        other => Err(format!("content type {other:?}, expected {JSON_CONTENT_TYPE}")),
    }
}

fn error_message(response: &ObservedResponse) -> Result<String, String> {
    check_json_content_type(response)?;
    let body = parse_json(&response.body)?;
    let is_single_field = body.as_object().map(|object| object.len() == 1).unwrap_or(false);
    match body["error"].as_str() {
        Some(message) if is_single_field => Ok(message.to_string()),
        _ => Err(format!("body is not {{\"error\": string}}: {body}")),
    }
}

fn check_exact_message(response: &ObservedResponse, expected: &str) -> Result<(), String> {
    let message = error_message(response)?;
    if message == expected {
        return Ok(());
    }
    Err(format!("error {message:?}, expected {expected:?}"))
}

fn check_message_contains(response: &ObservedResponse, expected: &str) -> Result<(), String> {
    let message = error_message(response)?;
    if message.contains(expected) {
        return Ok(());
    }
    Err(format!("error {message:?} does not contain {expected:?}"))
}

fn check_exact_json(response: &ObservedResponse, expected: &Value) -> Result<(), String> {
    check_json_content_type(response)?;
    let body = parse_json(&response.body)?;
    if &body == expected {
        return Ok(());
    }
    let text = body.to_string();
    let preview: String = text.chars().take(300).collect();
    Err(format!("body differs from expected JSON: {preview}"))
}

fn check_array_contains(response: &ObservedResponse, field: &str, value: &str) -> Result<(), String> {
    check_json_content_type(response)?;
    let body = parse_json(&response.body)?;
    let entries = body.as_array().ok_or_else(|| format!("body is not an array: {body}"))?;
    let found = entries.iter().any(|entry| entry[field] == value);
    if found {
        return Ok(());
    }
    Err(format!("no entry has {field} = {value:?}: {body}"))
}

fn check_first_line(lines: &Value, prefix: Option<&str>) -> Result<(), String> {
    let first = lines.get(0).and_then(Value::as_str);
    match (prefix, first) {
        (None, None) => Ok(()),
        (Some(prefix), Some(line)) if line.starts_with(prefix) => Ok(()),
        _ => Err(format!("lines {lines} do not start with {prefix:?}")),
    }
}

fn check_header(header: &Value, prefix: Option<&str>) -> Result<(), String> {
    let lines = header
        .as_array()
        .ok_or_else(|| format!("header is not an array: {header}"))?;
    let matches = match prefix {
        None => lines.is_empty(),
        Some(prefix) => lines.iter().any(|line| line_starts_with(line, prefix)),
    };
    if matches {
        return Ok(());
    }
    Err(format!("header does not match {prefix:?}: {} lines", lines.len()))
}

fn line_starts_with(line: &Value, prefix: &str) -> bool {
    match line.as_str() {
        Some(text) => text.starts_with(prefix),
        None => false,
    }
}

fn check_search(response: &ObservedResponse, expected: &ExpectedSearch) -> Result<(), String> {
    check_json_content_type(response)?;
    let body = parse_json(&response.body)?;
    let expected_coordinates = json!(expected.coordinates);
    if body["coordinates"] != expected_coordinates {
        return Err(format!("coordinates {}, expected {expected_coordinates}", body["coordinates"]));
    }
    let lines = body["lines"].as_array().ok_or_else(|| format!("lines is not an array: {body}"))?;
    if lines.len() != expected.first_line_prefixes.len() {
        return Err(format!("{} line lists, expected {}", lines.len(), expected.first_line_prefixes.len()));
    }
    for (coordinate_lines, prefix) in lines.iter().zip(&expected.first_line_prefixes) {
        check_first_line(coordinate_lines, *prefix)?;
    }
    check_header(&body["header"], expected.header_line_prefix)
}

fn check_empty(response: &ObservedResponse) -> Result<(), String> {
    if response.body.is_empty() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&response.body);
    Err(format!("expected an empty body, got {text}"))
}

fn check_body(response: &ObservedResponse, expected: &ExpectedBody) -> Result<(), String> {
    match expected {
        ExpectedBody::Json(value) => check_exact_json(response, value),
        ExpectedBody::ErrorMessage(message) => check_exact_message(response, message),
        ExpectedBody::ErrorMessageContaining(fragment) => check_message_contains(response, fragment),
        ExpectedBody::AnyErrorMessage => error_message(response).map(|_| ()),
        ExpectedBody::ArrayContaining { field, value } => check_array_contains(response, field, value),
        ExpectedBody::Search(search) => check_search(response, search),
        ExpectedBody::Empty => check_empty(response),
    }
}

fn check_status(response: &ObservedResponse, expected: u16) -> Result<(), String> {
    if response.status == expected {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&response.body);
    let preview: String = text.chars().take(300).collect();
    Err(format!("status {}, expected {expected}; body {preview}", response.status))
}

/// Checks one response against its case, describing every mismatch.
pub fn check_case(case: &ParityCase, response: &ObservedResponse) -> Result<(), String> {
    check_status(response, case.status)
        .and_then(|_| check_body(response, &case.expected))
        .map_err(|reason| format!("{}: {reason}", describe(case)))
}

/// Fails with every collected mismatch at once.
pub fn assert_no_failures(server: &str, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{server} failed {} parity case(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Checks the response of a binary note save, then reads the object straight
/// from the store and compares every byte.
pub async fn check_binary_note_saved(response: &ObservedResponse, path: &str) -> Result<(), String> {
    check_status(response, 204)?;
    check_empty(response)?;
    let store = StoreService::new();
    let bytes = store.get_object(path).await.map_err(|error| error.to_string())?;
    if bytes == every_byte() {
        return Ok(());
    }
    Err(format!("stored {} bytes differ from all 256 byte values", bytes.len()))
}

/// Writes all 256 byte values to `path` through `seqa_api::notes`, as the
/// other server or the Tauri app would.
pub async fn save_binary_note_with_core(path: &str) {
    let store = StoreService::new();
    let request = SaveNoteRequest {
        path: path.to_string(),
        data: EVERY_BYTE_BASE64.to_string(),
    };
    save_note(&store, &request).await.unwrap();
}

/// Checks that a `/notes/read` response carries all 256 byte values.
pub fn check_binary_note_read(response: &ObservedResponse) -> Result<(), String> {
    check_status(response, 200)?;
    let expected = json!({ "data": EVERY_BYTE_BASE64 });
    check_exact_json(response, &expected)
}

/// Reads `path` through `seqa_api::notes` and returns its base64.
pub async fn read_note_with_core(path: &str) -> String {
    let store = StoreService::new();
    let response = read_note(&store, path).await.unwrap();
    response.data
}

/// Fails when an object exists at `path`.
pub async fn check_not_written(path: &str) -> Result<(), String> {
    let store = StoreService::new();
    match store.get_object(path).await {
        Ok(bytes) => Err(format!("{path} was written ({} bytes)", bytes.len())),
        Err(_) => Ok(()),
    }
}

/// Distinct note paths for `count` concurrent saves by `server`.
pub fn concurrent_note_paths(server: &str, count: usize) -> Vec<String> {
    (0..count)
        .map(|number| note_path(&format!("{server}_parity_concurrent_{number}.bin")))
        .collect()
}

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

use ::serde::de::DeserializeOwned;
use ::serde::Serialize;
use lambda_http::http::Method;
use lambda_http::{Body, Error, Request, Response};
use seqa_api::list_directory_request::ListDirectoryRequest;
use seqa_api::local_uri::reject_local_uri;
use seqa_api::notes::notes_error::NotesError;
use seqa_api::notes::{self, ReadNoteRequest, SaveNoteRequest};
use seqa_api::search_request::SearchRequest;
use seqa_api::search_response::SearchResponse;
use seqa_core::api::search::search_features;
use seqa_core::reference_data;
use seqa_core::stores::buckets::{list_buckets, CloudBackend};
use seqa_core::stores::StoreService;
use seqa_core::utils::supported_formats;

type HandlerResult = Result<Response<Body>, Error>;

/// Routes an API Gateway request to its handler. The routes match `seqa_axum`;
/// a trailing `/` is ignored, and any other method or path returns a JSON 404.
///
/// # Errors
///
/// Returns an error only when a response cannot be built or serialized.
/// Request failures are returned as `Ok` responses with a JSON error body.
pub(crate) async fn function_handler(event: Request) -> HandlerResult {
    let method = event.method().clone();
    let path = event.uri().path().to_string();
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();

    match (&method, segments.as_slice()) {
        (&Method::POST, ["search"]) => search_handler(event).await,
        (&Method::POST, ["files", "list"]) => list_directory_handler(event).await,
        (&Method::GET, ["files", "buckets", backend]) => buckets_handler(backend).await,
        (&Method::GET, ["files", "formats"]) => json_response(200, &supported_formats()),
        (&Method::GET, ["genes", "symbols", genome, source]) => gene_symbols(genome, source),
        (&Method::GET, ["cytobands", genome, chromosome]) => cytobands(genome, chromosome),
        (&Method::POST, ["notes", "read"]) => read_note_handler(event).await,
        (&Method::POST, ["notes", "save"]) => save_note_handler(event).await,
        _ => error_response(404, &format!("Not found: {} {}", method, path)),
    }
}

async fn search_handler(event: Request) -> HandlerResult {
    let request = match parse_payload::<SearchRequest>(&event) {
        Ok(request) => request,
        Err(response) => return *response,
    };
    let options = match request.to_search_options() {
        Ok(options) => options,
        Err(error) => return error_response(400, &error.to_string()),
    };

    let store = StoreService::new();
    let result = match search_features(&store, &options).await {
        Ok(result) => result,
        Err(error) => return error_response(500, &format!("Search Failed: {}", error)),
    };
    let response = SearchResponse::from_result(result, &options);
    json_response(200, &response)
}

async fn list_directory_handler(event: Request) -> HandlerResult {
    let request = match parse_payload::<ListDirectoryRequest>(&event) {
        Ok(request) => request,
        Err(response) => return *response,
    };
    if let Err(response) = check_not_local(&request.uri) {
        return *response;
    }

    let store = StoreService::new();
    match store.list_directory(&request.uri).await {
        Ok(entries) => json_response(200, &entries),
        Err(error) => error_response(500, &format!("Error listing {}: {}", request.uri, error)),
    }
}

async fn buckets_handler(backend: &str) -> HandlerResult {
    let backend = match CloudBackend::from_id(backend) {
        Ok(backend) => backend,
        Err(error) => return error_response(400, &error.to_string()),
    };
    match list_buckets(backend).await {
        Ok(buckets) => json_response(200, &buckets),
        Err(error) => error_response(500, &format!("Error listing buckets: {}", error)),
    }
}

fn gene_symbols(genome: &str, source: &str) -> HandlerResult {
    json_response(200, &reference_data::gene_coordinates(genome, source))
}

fn cytobands(genome: &str, chromosome: &str) -> HandlerResult {
    json_response(200, &reference_data::cytobands(genome, chromosome))
}

async fn read_note_handler(event: Request) -> HandlerResult {
    let request = match parse_payload::<ReadNoteRequest>(&event) {
        Ok(request) => request,
        Err(response) => return *response,
    };
    if let Err(response) = check_not_local(&request.path) {
        return *response;
    }

    let store = StoreService::new();
    match notes::read_note(&store, &request.path).await {
        Ok(response) => json_response(200, &response),
        Err(error) => notes_error_response(&error),
    }
}

async fn save_note_handler(event: Request) -> HandlerResult {
    let request = match parse_payload::<SaveNoteRequest>(&event) {
        Ok(request) => request,
        Err(response) => return *response,
    };
    if let Err(response) = check_not_local(&request.path) {
        return *response;
    }

    let store = StoreService::new();
    match notes::save_note(&store, &request).await {
        Ok(()) => empty_response(204),
        Err(error) => notes_error_response(&error),
    }
}

fn parse_payload<T: DeserializeOwned>(event: &Request) -> Result<T, Box<HandlerResult>> {
    if !has_json_content_type(event) {
        return Err(bad_request("Invalid request body: content type must be application/json"));
    }
    serde_json::from_slice(event.body())
        .map_err(|error| bad_request(&format!("Invalid request body: {}", error)))
}

fn has_json_content_type(event: &Request) -> bool {
    let content_type = event
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok());
    match content_type {
        Some(content_type) => is_json_media_type(content_type),
        None => false,
    }
}

fn is_json_media_type(content_type: &str) -> bool {
    let media_type = content_type.split(';').next().unwrap_or_default();
    media_type.trim().eq_ignore_ascii_case("application/json")
}

fn check_not_local(uri: &str) -> Result<(), Box<HandlerResult>> {
    reject_local_uri(uri).map_err(|error| bad_request(&error.to_string()))
}

fn bad_request(message: &str) -> Box<HandlerResult> {
    let response = error_response(400, message);
    Box::new(response)
}

fn notes_error_status(error: &NotesError) -> u16 {
    match error {
        NotesError::InvalidPath(_) | NotesError::InvalidData { .. } => 400,
        NotesError::NotFound(_) => 404,
        NotesError::Store { .. } => 500,
    }
}

fn notes_error_response(error: &NotesError) -> HandlerResult {
    let status = notes_error_status(error);
    error_response(status, &error.to_string())
}

fn json_response<T: Serialize>(status: u16, value: &T) -> HandlerResult {
    let body = serde_json::to_string(value)?;
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::Text(body))
        .map_err(Box::new)?)
}

fn empty_response(status: u16) -> HandlerResult {
    Ok(Response::builder()
        .status(status)
        .body(Body::Empty)
        .map_err(Box::new)?)
}

fn error_response(status: u16, message: &str) -> HandlerResult {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::Text(
            serde_json::json!({ "error": message }).to_string(),
        ))
        .map_err(Box::new)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::{Body, Request};
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::indexes::index_cache::IndexGuard;

    const S3_NOTES: &str = "s3://com.gmail.docarw/test_data/notes";
    const HELLO_WORLD_BASE64: &str = "SGVsbG8gd29ybGQ=";

    fn build_request(options: &SearchOptions) -> Request {
        let payload = SearchRequest {
            path: options.file_path.clone(),
            coordinates: options
                .coordinates
                .iter()
                .map(|coordinate| {
                    format!(
                        "{}:{}-{}",
                        coordinate.get_chromosome(),
                        coordinate.get_begin(),
                        coordinate.get_end()
                    )
                })
                .collect(),
            with_header: Some(options.include_header),
            only_header: Some(options.header_only),
            index_path: Some(options.index_path.clone()),
            output_format: Some(options.output_format.to_string()),
        };
        let body = serde_json::to_string(&payload).unwrap();
        post_request("/search", &body)
    }

    fn post_request(uri: &str, body: &str) -> Request {
        let mut request = Request::new(Body::Text(body.to_string()));
        *request.method_mut() = Method::POST;
        *request.uri_mut() = uri.parse().unwrap();
        request
            .headers_mut()
            .insert("content-type", "application/json".parse().unwrap());
        request
    }

    fn get_request(uri: &str) -> Request {
        let mut req = Request::new(Body::Empty);
        *req.method_mut() = Method::GET;
        *req.uri_mut() = uri.parse().unwrap();
        req
    }

    async fn send(request: Request) -> Response<Body> {
        function_handler(request).await.unwrap()
    }

    fn body_string(response: &Response<Body>) -> String {
        String::from_utf8(response.body().to_vec()).unwrap()
    }

    fn assert_json_error(response: &Response<Body>, status: u16) -> String {
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["content-type"], "application/json");
        let body = body_string(response);
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let message = json["error"].as_str();
        assert!(message.is_some(), "{body}");
        message.unwrap().to_string()
    }

    fn read_note_request(path: &str) -> Request {
        let body = format!(r#"{{"path":"{path}"}}"#);
        post_request("/notes/read", &body)
    }

    fn save_note_request(path: &str, data: &str) -> Request {
        let body = format!(r#"{{"path":"{path}","data":"{data}"}}"#);
        post_request("/notes/save", &body)
    }

    #[tokio::test]
    async fn bigwig_chr12() {
        let options = SearchOptions::new(
            "s3://com.gmail.docarw/test_data/density.bw",
            &vec!["chr12".to_string()],
        )
        .unwrap()
        .set_index_path("-")
        .set_output_format("bigwig")
        .set_include_header(false);

        let response = function_handler(build_request(&options)).await.unwrap();
        assert_eq!(response.status(), 200);

        let result: SearchResponse = serde_json::from_slice(&response.body().to_vec()).unwrap();
        let lines = result.lines.concat();
        assert!(!lines.is_empty(), "expected BigWig chr12 results");

        for line in &lines {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields[0], "chr12",
                "unexpected chromosome in line: {}",
                line
            );
            let begin: u32 = fields[1].parse().unwrap();
            let end: u32 = fields[2].parse().unwrap();
            assert!(begin < end, "expected begin < end, got {}-{}", begin, end);
        }
    }

    #[tokio::test]
    async fn bam_chr2_range() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12877.bam.bai");
        let options = SearchOptions::new(
            "s3://com.gmail.docarw/test_data/NA12877.bam",
            &vec!["chr2:50000000-50000100".to_string()],
        )
        .unwrap()
        .set_index_path("s3://com.gmail.docarw/test_data/NA12877.bam.bai")
        .set_output_format("bam")
        .set_include_header(false);

        let response = function_handler(build_request(&options)).await.unwrap();
        assert_eq!(response.status(), 200);

        let result: SearchResponse = serde_json::from_slice(&response.body().to_vec()).unwrap();
        let lines = result.lines.concat();
        assert!(!lines.is_empty(), "expected BAM chr2 results");

        for line in &lines {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields[2], "chr2", "unexpected reference in line: {}", line);
        }
    }

    #[tokio::test]
    async fn header_only_when_requested() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12877.bam.bai");
        let mut options = SearchOptions::new(
            "s3://com.gmail.docarw/test_data/NA12877.bam",
            &vec!["chr2:50000000-50000100".to_string()],
        )
        .unwrap()
        .set_index_path("s3://com.gmail.docarw/test_data/NA12877.bam.bai")
        .set_output_format("bam")
        .set_include_header(false);

        let response = function_handler(build_request(&options)).await.unwrap();
        assert_eq!(response.status(), 200);
        let result: SearchResponse = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert!(result.header.is_empty());
        assert_eq!(result.lines.len(), 1);

        options = options.set_include_header(true);
        let response = function_handler(build_request(&options)).await.unwrap();
        assert_eq!(response.status(), 200);
        let result: SearchResponse = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert!(!result.header.is_empty());
        assert_eq!(result.lines.len(), 1);
    }

    #[tokio::test]
    async fn search_several_coordinates() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi");
        let request = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": ["chr1:116549-116549", "chr1:1-2"]}"#,
        );
        let response = function_handler(request).await.unwrap();
        assert_eq!(response.status(), 200);
        let result: SearchResponse = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert_eq!(result.coordinates.len(), 2);
        assert!(result.coordinates[0].starts_with("chr1:116549"));
        assert_eq!(result.coordinates[1], "chr1:1-2");
        assert_eq!(result.lines.len(), 2);
        assert!(!result.lines[0].is_empty());
        assert!(result.lines[1].is_empty());
    }

    #[tokio::test]
    async fn search_with_header_includes_chrom_line() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi");
        let request = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": ["chr1:116549-116549"], "with_header": true}"#,
        );
        let response = send(request).await;
        assert_eq!(response.status(), 200);
        let result: SearchResponse = serde_json::from_slice(response.body()).unwrap();
        assert!(result.header.iter().any(|line| line.starts_with("#CHROM")));
    }

    #[tokio::test]
    async fn search_rejects_empty_coordinates() {
        let request = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": []}"#,
        );
        let response = function_handler(request).await.unwrap();
        assert_eq!(response.status(), 400);
        let body = String::from_utf8(response.body().to_vec()).unwrap();
        assert_eq!(body, r#"{"error":"No coordinates"}"#);
    }

    #[tokio::test]
    async fn search_rejects_single_coordinate_string() {
        let request = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": "chr1:1-1000"}"#,
        );
        let response = function_handler(request).await.unwrap();
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn search_rejects_malformed_body() {
        let response = send(post_request("/search", "not json")).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn search_rejects_unsupported_extension() {
        let request = post_request(
            "/search",
            r#"{"path": "s3://bucket/file.xyz", "coordinates": ["chr1:1-10"]}"#,
        );
        let response = send(request).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn search_rejects_unknown_output_format() {
        let request = post_request(
            "/search",
            r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"], "output_format": "nonsense"}"#,
        );
        let response = send(request).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn search_uses_index_path_instead_of_inferred_index() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz.tbi");
        let inferred = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000"]}"#,
        );
        let response = send(inferred).await;
        assert_eq!(response.status(), 200);

        let request = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000"], "index_path": "s3://com.gmail.docarw/test_data/no_such_index.tbi"}"#,
        );
        let response = send(request).await;
        let message = assert_json_error(&response, 500);
        assert!(message.contains("no_such_index.tbi"), "{message}");
    }

    #[tokio::test]
    async fn search_uses_output_format_instead_of_extension() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz.tbi");
        let inferred = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000000"]}"#,
        );
        let response = send(inferred).await;
        assert_eq!(response.status(), 200);

        let request = post_request(
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000000"], "output_format": "bedgraph"}"#,
        );
        let response = send(request).await;
        assert_json_error(&response, 500);
    }

    #[tokio::test]
    async fn search_only_at_search_path() {
        let request = post_request(
            "/",
            r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"]}"#,
        );
        let response = send(request).await;
        assert_json_error(&response, 404);
    }

    #[tokio::test]
    async fn wrong_method_is_json_404() {
        let response = send(get_request("/search")).await;
        let message = assert_json_error(&response, 404);
        assert_eq!(message, "Not found: GET /search");
    }

    #[tokio::test]
    async fn not_found_message() {
        let response = send(post_request("/anything", "{}")).await;
        let message = assert_json_error(&response, 404);
        assert_eq!(message, "Not found: POST /anything");
    }

    #[tokio::test]
    async fn cytobands_empty_genome_is_routed() {
        let response = send(get_request("/cytobands//chr1")).await;
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn list_directory_rejects_control_character_file_uri() {
        let response = send(post_request("/files/list", r#"{"uri":"fi\tle:///etc"}"#)).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn read_control_character_local_note() {
        let request = post_request(
            "/notes/read",
            r#"{"path":"fi\tle:///home/user/analysis/notes/today.txt"}"#,
        );
        let response = send(request).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn unknown_post_is_json_404() {
        let response = send(post_request("/anything", "{}")).await;
        assert_json_error(&response, 404);
    }

    #[tokio::test]
    async fn removed_recursive_listing() {
        let response = send(post_request("/files", r#""s3://com.gmail.docarw/test_data/""#)).await;
        assert_json_error(&response, 404);
    }

    #[tokio::test]
    async fn old_cytoband_path_is_404() {
        let response = send(get_request("/genes/cytobands/grch38/chr1")).await;
        assert_json_error(&response, 404);
    }

    #[tokio::test]
    async fn gene_coordinates_brca2_grch38() {
        let response = function_handler(get_request("/genes/symbols/grch38/refseq"))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let genes: serde_json::Value = serde_json::from_slice(&response.body().to_vec()).unwrap();
        let gene_arr = genes.as_array().unwrap();
        let brca2 = gene_arr.iter().find(|g| {
            g.as_object().unwrap().get("symbol").unwrap().to_owned() == "BRCA2".to_string()
        });
        assert!(!brca2.is_none());
        if let Some(b) = brca2 {
            assert_eq!(b.as_object().unwrap().get("symbol").unwrap(), "BRCA2");
        }
    }

    #[tokio::test]
    async fn gene_symbols_trailing_slash_optional() {
        let with_slash = send(get_request("/genes/symbols/grch38/refseq/")).await;
        let without_slash = send(get_request("/genes/symbols/grch38/refseq")).await;
        assert_eq!(with_slash.status(), 200);
        assert_eq!(without_slash.status(), 200);
        assert_eq!(body_string(&with_slash), body_string(&without_slash));
    }

    #[tokio::test]
    async fn cytobands_chr1_grch38() {
        let response = function_handler(get_request("/cytobands/grch38/chr1"))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let bands: serde_json::Value = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert!(bands.as_array().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn cytobands_chr1_hg19() {
        let response = function_handler(get_request("/cytobands/hg19/chr1"))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let bands: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
        let bands = bands.as_array().unwrap();
        assert!(!bands.is_empty());
        assert!(bands.iter().all(|band| band["chromosome"] == "chr1"));
    }

    #[tokio::test]
    async fn cytobands_trailing_slash() {
        let response = send(get_request("/cytobands/grch38/chr1/")).await;
        assert_eq!(response.status(), 200);
        let bands: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
        let bands = bands.as_array().unwrap();
        assert!(!bands.is_empty());
        assert!(bands.iter().all(|band| band["chromosome"] == "chr1"));
    }

    #[tokio::test]
    async fn cytobands_unknown_chromosome_is_empty() {
        let response = function_handler(get_request("/cytobands/grch38/chrNotReal"))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let bands: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(bands, serde_json::json!([]));
    }

    #[tokio::test]
    async fn list_directory_s3() {
        let directory = format!("{S3_NOTES}/lambda_listing/");
        let paths: Vec<String> = ["one.txt", "two.txt", "three.txt"]
            .iter()
            .map(|file_name| format!("{directory}{file_name}"))
            .collect();
        let _cleanup: Vec<DeleteOnDrop> = paths.iter().map(|path| DeleteOnDrop::new(path)).collect();
        for path in &paths {
            let saved = send(save_note_request(path, HELLO_WORLD_BASE64)).await;
            assert_eq!(saved.status(), 204);
        }

        let body = format!(r#"{{"uri":"{directory}"}}"#);
        let response = send(post_request("/files/list", &body)).await;
        assert_eq!(response.status(), 200);
        let entries: Vec<serde_json::Value> = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(entries.len(), paths.len());
        for path in &paths {
            let listed = entries.iter().any(|entry| entry["uri"] == path.as_str());
            assert!(listed, "{path} not in {entries:?}");
        }
    }

    #[tokio::test]
    async fn list_directory_rejects_local_uri() {
        let response = send(post_request("/files/list", r#"{"uri":"file:///etc"}"#)).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn list_directory_rejects_scheme_less_path() {
        let response = send(post_request("/files/list", r#"{"uri":"/etc/passwd"}"#)).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn list_directory_rejects_malformed_body() {
        let response = send(post_request("/files/list", r#"{"wrong":"field"}"#)).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn list_buckets_rejects_unknown_backend() {
        let response = send(get_request("/files/buckets/nonsense")).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn list_buckets_rejects_local_backend() {
        let response = send(get_request("/files/buckets/local")).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn file_formats_include_bam() {
        let response = send(get_request("/files/formats")).await;
        assert_eq!(response.status(), 200);
        let body = body_string(&response);
        assert!(body.contains("\"format\":\"BAM\""), "{body}");
    }

    #[tokio::test]
    async fn save_then_read_note() {
        let path = format!("{S3_NOTES}/lambda_round_trip.txt");
        let _cleanup = DeleteOnDrop::new(&path);
        let saved = send(save_note_request(&path, HELLO_WORLD_BASE64)).await;
        assert_eq!(saved.status(), 204);
        assert!(matches!(saved.body(), Body::Empty));

        let read = send(read_note_request(&path)).await;
        assert_eq!(read.status(), 200);
        assert_eq!(body_string(&read), r#"{"data":"SGVsbG8gd29ybGQ="}"#);
    }

    #[tokio::test]
    async fn read_missing_note() {
        let path = format!("{S3_NOTES}/lambda_does_not_exist.txt");
        let response = send(read_note_request(&path)).await;
        assert_json_error(&response, 404);
    }

    #[tokio::test]
    async fn read_note_outside_notes_directory() {
        let response = send(read_note_request("s3://com.gmail.docarw/test.txt")).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn read_local_note() {
        let response = send(read_note_request("file:///home/user/analysis/notes/today.txt")).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn save_scheme_less_note() {
        let response = send(save_note_request("/analysis/notes/today.txt", HELLO_WORLD_BASE64)).await;
        assert_json_error(&response, 400);
    }

    #[tokio::test]
    async fn save_note_outside_notes_directory() {
        let path = "s3://com.gmail.docarw/test_data/lambda_never_written.txt";
        let _cleanup = DeleteOnDrop::new(path);
        let response = send(save_note_request(path, HELLO_WORLD_BASE64)).await;
        assert_json_error(&response, 400);
        assert_eq!(http_parity::check_not_written(path).await, Ok(()));
    }

    #[tokio::test]
    async fn save_note_bad_base64() {
        let path = format!("{S3_NOTES}/lambda_never_written_bad_data.txt");
        let _cleanup = DeleteOnDrop::new(&path);
        let saved = send(save_note_request(&path, "not base64!")).await;
        assert_json_error(&saved, 400);

        let read = send(read_note_request(&path)).await;
        assert_eq!(read.status(), 404);
    }

    #[tokio::test]
    async fn read_note_malformed_body() {
        let not_json = send(post_request("/notes/read", "not json")).await;
        assert_json_error(&not_json, 400);

        let missing_path = send(post_request("/notes/read", r#"{"wrong":"field"}"#)).await;
        assert_json_error(&missing_path, 400);
    }

    #[tokio::test]
    async fn save_note_malformed_body() {
        let not_json = send(post_request("/notes/save", "not json")).await;
        assert_json_error(&not_json, 400);

        let no_data_path = format!("{S3_NOTES}/lambda_never_written_no_data.txt");
        let _cleanup = DeleteOnDrop::new(&no_data_path);
        let body = format!(r#"{{"path":"{no_data_path}"}}"#);
        let missing_data = send(post_request("/notes/save", &body)).await;
        assert_json_error(&missing_data, 400);
        assert_eq!(http_parity::check_not_written(&no_data_path).await, Ok(()));
    }

    mod http_parity {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/http_parity.rs"));
    }

    use http_parity::{DeleteOnDrop, ObservedResponse, ParityRequest};
    use tokio::task::JoinSet;

    fn parity_request(request: &ParityRequest) -> Request {
        let mut lambda_request = Request::new(Body::Text(request.body.clone()));
        *lambda_request.method_mut() = request.method.parse().unwrap();
        *lambda_request.uri_mut() = request.path.parse().unwrap();
        if let Some(content_type) = request.content_type {
            let value = content_type.parse().unwrap();
            lambda_request.headers_mut().insert("content-type", value);
        }
        lambda_request
    }

    fn observe(response: &Response<Body>) -> ObservedResponse {
        let content_type = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_string());
        ObservedResponse {
            status: response.status().as_u16(),
            content_type,
            body: response.body().to_vec(),
        }
    }

    async fn send_parity(request: &ParityRequest) -> ObservedResponse {
        let lambda_request = parity_request(request);
        let response = send(lambda_request).await;
        observe(&response)
    }

    fn index_guards() -> Vec<IndexGuard> {
        http_parity::indexes_used()
            .into_iter()
            .map(IndexGuard::new)
            .collect()
    }

    #[tokio::test]
    async fn parity_table() {
        let _guards = index_guards();
        let _cleanup = http_parity::rejected_save_guards();
        let mut cases = http_parity::parity_cases();
        cases.push(http_parity::head_case(false));
        let mut failures = Vec::new();
        for case in cases {
            let response = send_parity(&case.request).await;
            if let Err(failure) = http_parity::check_case(&case, &response) {
                failures.push(failure);
            }
        }
        failures.extend(http_parity::rejected_saves_written().await);
        http_parity::assert_no_failures("seqa_lambda", &failures);
    }

    #[tokio::test]
    async fn parity_binary_note_saved_over_http_is_byte_exact() {
        let path = http_parity::note_path("lambda_parity_binary.bin");
        let _cleanup = DeleteOnDrop::new(&path);
        let request = http_parity::save_note_request(&path, http_parity::EVERY_BYTE_BASE64);
        let response = send_parity(&request).await;
        let checked = http_parity::check_binary_note_saved(&response, &path).await;
        assert_eq!(checked, Ok(()));
        let core_data = http_parity::read_note_with_core(&path).await;
        assert_eq!(core_data, http_parity::EVERY_BYTE_BASE64);
    }

    #[tokio::test]
    async fn parity_binary_note_saved_by_core_reads_over_http() {
        let path = http_parity::note_path("lambda_parity_written_by_core.bin");
        let _cleanup = DeleteOnDrop::new(&path);
        http_parity::save_binary_note_with_core(&path).await;
        let request = http_parity::read_note_request(&path);
        let response = send_parity(&request).await;
        assert_eq!(http_parity::check_binary_note_read(&response), Ok(()));
    }

    #[tokio::test]
    async fn parity_form_encoded_save_is_not_written() {
        let path = http_parity::note_path("lambda_parity_never_written_form.txt");
        let _cleanup = DeleteOnDrop::new(&path);
        let request = http_parity::form_save_note_request(&path);
        let response = send_parity(&request).await;
        assert_eq!(response.status, 400);
        let written = http_parity::check_not_written(&path).await;
        assert_eq!(written, Ok(()));
    }

    #[tokio::test]
    async fn parity_concurrent_note_saves() {
        let paths = http_parity::concurrent_note_paths("lambda", 8);
        let _cleanup: Vec<DeleteOnDrop> = paths.iter().map(|path| DeleteOnDrop::new(path)).collect();
        let mut saves = JoinSet::new();
        for path in &paths {
            let request = http_parity::save_note_request(path, http_parity::EVERY_BYTE_BASE64);
            let lambda_request = parity_request(&request);
            saves.spawn(async move { function_handler(lambda_request).await.unwrap().status() });
        }
        let statuses = saves.join_all().await;
        assert!(statuses.iter().all(|status| *status == 204), "{statuses:?}");
        for path in &paths {
            let data = http_parity::read_note_with_core(path).await;
            assert_eq!(data, http_parity::EVERY_BYTE_BASE64, "{path}");
        }
    }
}

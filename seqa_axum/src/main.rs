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

use crate::cache::AppCache;
use axum::{
    Json, Router, ServiceExt,
    extract::{Path, Request, State, rejection::JsonRejection},
    http::{Method, StatusCode, Uri, header},
    response::IntoResponse,
    routing::{get, post},
};
use seqa_api::list_directory_request::ListDirectoryRequest;
use seqa_api::local_uri::{LocalUriError, reject_local_uri};
use seqa_api::notes::notes_error::NotesError;
use seqa_api::notes::{self, ReadNoteRequest, ReadNoteResponse, SaveNoteRequest};
use seqa_api::search_request::SearchRequest;
use seqa_api::search_response::SearchResponse;
use seqa_core::api::output_format::OutputFormat;
use seqa_core::api::search::search_features;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::models::cytoband::Cytoband;
use seqa_core::models::gene_coordinate::GeneCoordinate;
use seqa_core::reference_data;
use seqa_core::stores::buckets::{CloudBackend, list_buckets as list_cloud_buckets};
use seqa_core::stores::{DirectoryEntry, StoreService};
use seqa_core::TabixHeader;
use seqa_core::utils::{FileFormat, supported_formats};
use serde::Serialize;
use std::any::Any;
use std::sync::Arc;
use thiserror::Error;
use tower_http::cors::CorsLayer;
use tower_http::normalize_path::NormalizePath;
use seqa_core::BamHeader;

pub mod cache;

/// State shared by every handler: one store service, and the cache of parsed
/// indexes and headers that later searches reuse.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<StoreService>,
    pub cache: AppCache,
}

/// A failed request. Each variant holds the message and becomes a response
/// with its status code and the body `{"error": "<message>"}`.
#[derive(Debug, Error)]
pub enum ApiError {
    /// 404: unknown route, or a note with no object.
    #[error("{0}")]
    NotFound(String),
    /// 400: malformed or non-JSON body, invalid search request, local URI,
    /// unknown backend, or invalid note path or data.
    #[error("{0}")]
    BadRequest(String),
    /// 500: the search or store failed after the request was validated.
    #[error("{0}")]
    InternalError(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = ErrorBody {
            error: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

impl From<NotesError> for ApiError {
    fn from(error: NotesError) -> Self {
        let message = error.to_string();
        match error {
            NotesError::InvalidPath(_) | NotesError::InvalidData { .. } => {
                ApiError::BadRequest(message)
            }
            NotesError::NotFound(_) => ApiError::NotFound(message),
            NotesError::Store { .. } => ApiError::InternalError(message),
        }
    }
}

impl From<LocalUriError> for ApiError {
    fn from(error: LocalUriError) -> Self {
        ApiError::BadRequest(error.to_string())
    }
}

fn invalid_payload(rejection: JsonRejection) -> ApiError {
    ApiError::BadRequest(format!("Invalid JSON payload: {}", rejection))
}

async fn feature_search(
    State(state): State<AppState>,
    payload: Result<Json<SearchRequest>, JsonRejection>,
) -> Result<Json<SearchResponse>, ApiError> {
    let Json(request) = payload.map_err(invalid_payload)?;
    let mut search_options = request
        .to_search_options()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    populate_from_cache(&state.cache, &mut search_options).await;
    let result = search_features(&state.store, &search_options)
        .await
        .map_err(|e| ApiError::InternalError(format!("Search Failed: {}", e)))?;
    write_back_to_cache(&state.cache, &search_options, &result).await;

    let response = SearchResponse::from_result(result, &search_options);
    Ok(Json(response))
}

async fn populate_from_cache(cache: &AppCache, options: &mut SearchOptions) {
    match options.output_format {
        OutputFormat::BAM => {
            options.bam_index = cache.bai.get(&options.index_path).await;
            options.bam_header = cache.bam_header.get(&options.file_path).await;
        }
        OutputFormat::VCF
        | OutputFormat::BED
        | OutputFormat::BEDGRAPH
        | OutputFormat::GFF
        | OutputFormat::GTF => {
            options.tabix_index = cache.tabix.get(&options.index_path).await;
            options.tabix_header = cache.tabix_header.get(&options.file_path).await;
        }
        OutputFormat::FASTA => {
            options.fasta_index = cache.fai.get(&options.index_path).await;
        }
        _ => {}
    }
}

async fn write_back_to_cache(
    cache: &AppCache,
    options: &SearchOptions,
    result: &seqa_core::api::search_result::SearchResult,
) {
    if let Some(bai) = &result.bam_index {
        cache
            .bai
            .insert(options.index_path.clone(), bai.clone())
            .await;
    }
    if let Some(header) = &result.header
        && options.index_path.ends_with(".bai")
    {
        let any_box: &dyn Any = header.as_ref();
        if let Some(header) = any_box.downcast_ref::<BamHeader>() {
            cache
                .bam_header
                .insert(options.file_path.clone(), header.clone())
                .await;
        }
    }
    if let Some(tabix) = &result.tabix_index {
        cache
            .tabix
            .insert(options.index_path.clone(), tabix.clone())
            .await;
    }
    if let Some(header) = &result.header
        && options.index_path.ends_with(".tbi")
    {
        let any_box: &dyn Any = header.as_ref();
        if let Some(header) = any_box.downcast_ref::<TabixHeader>() {
            cache
                .tabix_header
                .insert(options.file_path.clone(), header.clone())
                .await;
        }
    }
    if let Some(fai) = &result.fasta_index {
        cache
            .fai
            .insert(options.index_path.clone(), fai.clone())
            .await;
    }
}

async fn get_gene_symbols(
    Path((genome, source)): Path<(String, String)>,
) -> Json<&'static [GeneCoordinate]> {
    Json(reference_data::gene_coordinates(&genome, &source))
}

async fn get_cytobands(
    Path((genome, chromosome)): Path<(String, String)>,
) -> Json<Vec<Cytoband>> {
    Json(reference_data::cytobands(&genome, &chromosome))
}

/// Lists one level of a directory or bucket prefix. Local URIs are rejected.
async fn list_directory(
    State(state): State<AppState>,
    payload: Result<Json<ListDirectoryRequest>, JsonRejection>,
) -> Result<Json<Vec<DirectoryEntry>>, ApiError> {
    let Json(request) = payload.map_err(invalid_payload)?;
    reject_local_uri(&request.uri)?;

    let entries = state
        .store
        .list_directory(&request.uri)
        .await
        .map_err(|e| ApiError::InternalError(format!("Error listing {}: {}", request.uri, e)))?;

    Ok(Json(entries))
}

async fn list_buckets(Path(backend): Path<String>) -> Result<Json<Vec<String>>, ApiError> {
    let backend =
        CloudBackend::from_id(&backend).map_err(|e| ApiError::BadRequest(format!("{}", e)))?;

    let buckets = list_cloud_buckets(backend)
        .await
        .map_err(|e| ApiError::InternalError(format!("Error listing buckets: {}", e)))?;

    Ok(Json(buckets))
}

async fn get_file_formats() -> Json<Vec<FileFormat>> {
    Json(supported_formats())
}

async fn read_note(
    State(state): State<AppState>,
    payload: Result<Json<ReadNoteRequest>, JsonRejection>,
) -> Result<Json<ReadNoteResponse>, ApiError> {
    let Json(request) = payload.map_err(invalid_payload)?;
    reject_local_uri(&request.path)?;
    let response = notes::read_note(&state.store, &request.path).await?;
    Ok(Json(response))
}

async fn save_note(
    State(state): State<AppState>,
    payload: Result<Json<SaveNoteRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(request) = payload.map_err(invalid_payload)?;
    reject_local_uri(&request.path)?;
    notes::save_note(&state.store, &request).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn not_found_fallback(method: Method, uri: Uri) -> ApiError {
    ApiError::NotFound(format!("Not found: {} {}", method, uri.path()))
}

/// Builds the seqa HTTP service: every route, CORS, JSON 404s for unknown
/// paths and methods, and an optional trailing slash on every path.
pub fn app() -> NormalizePath<Router> {
    let local_origins = [
        "http://localhost:5173".parse().unwrap(),
        "http://localhost:6006".parse().unwrap(),
        "http://localhost:6007".parse().unwrap(),
        "http://127.0.0.1:6007".parse().unwrap(),
    ];

    let cors = CorsLayer::new()
        .allow_origin(local_origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true);

    let state = AppState {
        store: Arc::new(StoreService::new()),
        cache: AppCache::new(),
    };

    let router = Router::new()
        .route("/search", post(feature_search))
        .route("/files/list", post(list_directory))
        .route("/files/buckets/{backend}", get(list_buckets))
        .route("/files/formats", get(get_file_formats))
        .route("/genes/symbols/{genome}/{source}", get(get_gene_symbols))
        .route("/cytobands/{genome}/{chromosome}", get(get_cytobands))
        .route("/notes/read", post(read_note))
        .route("/notes/save", post(save_note))
        .fallback(not_found_fallback)
        .method_not_allowed_fallback(not_found_fallback)
        .layer(cors)
        .with_state(state);

    NormalizePath::trim_trailing_slash(router)
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8000";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    println!("seqa_axum listening on http://{}", addr);
    let service = ServiceExt::<Request>::into_make_service(app());
    axum::serve(listener, service).await.expect("server error");
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use http_body_util::BodyExt;
    use seqa_core::indexes::index_cache::IndexGuard;
    use tower::ServiceExt;

    fn json_request(method: &str, uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn get_request(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    async fn send(request: Request<Body>) -> axum::response::Response {
        app().oneshot(request).await.unwrap()
    }

    async fn assert_json_error(response: axum::response::Response, status: StatusCode) -> String {
        assert_eq!(response.status(), status);
        let content_type = response.headers()[header::CONTENT_TYPE].clone();
        assert_eq!(content_type, "application/json");
        let body = body_string(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let message = json["error"].as_str();
        assert!(message.is_some(), "{body}");
        message.unwrap().to_string()
    }

    async fn body_string(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_index() {
        let response = send(get_request("/")).await;
        let message = assert_json_error(response, StatusCode::NOT_FOUND).await;
        assert_eq!(message, "Not found: GET /");
    }

    #[tokio::test]
    async fn test_search_unsupported_file_type() {
        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://bucket/file.txt", "coordinates": ["chr1:1-1000"]}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_search_vcf() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi");
        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": ["chr1:116549-116549"]}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        // The body is JSON, so the tab separator is escaped as a literal `\t`.
        assert!(body.contains(r"chr1\t116549"));
    }

    #[tokio::test]
    async fn test_search_several_coordinates() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi");
        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": ["chr1:116549-116549", "chr1:1-2"]}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains(r#""coordinates":["chr1:116549","chr1:1-2"]"#), "{body}");
        // lines is the last field: one list per coordinate, the second region has no variants
        assert!(body.contains(r"chr1\t116549"));
        assert!(body.ends_with(r#"],[]]}"#));
    }

    #[tokio::test]
    async fn test_search_header_only_when_requested() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi");

        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": ["chr1:116549-116549"]}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains(r#""header":[]"#));

        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": ["chr1:116549-116549"], "with_header": true}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("##fileformat=VCF"));
    }

    #[tokio::test]
    async fn test_write_back_caches_headers() {
        use super::{AppCache, write_back_to_cache};
        use seqa_core::api::search::search_features;
        use seqa_core::api::search_options::SearchOptions;
        use seqa_core::stores::StoreService;

        let store = StoreService::new();
        let cache = AppCache::new();

        let vcf = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz";
        let vcf_index = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi";
        let _vcf_guard = IndexGuard::new(vcf_index);
        let options = SearchOptions::new(vcf, &["chr1:116549-116549".to_string()])
            .unwrap()
            .set_index_path(vcf_index);
        let result = search_features(&store, &options).await.unwrap();
        write_back_to_cache(&cache, &options, &result).await;
        assert!(cache.tabix_header.get(vcf).await.is_some());

        let bam = "s3://com.gmail.docarw/test_data/NA12877.bam";
        let bam_index = "s3://com.gmail.docarw/test_data/NA12877.bam.bai";
        let _bam_guard = IndexGuard::new(bam_index);
        let options = SearchOptions::new(bam, &["chr1:10000000-10000000".to_string()])
            .unwrap()
            .set_index_path(bam_index);
        let result = search_features(&store, &options).await.unwrap();
        write_back_to_cache(&cache, &options, &result).await;
        assert!(cache.bam_header.get(bam).await.is_some());
    }

    #[tokio::test]
    async fn test_list_directory_s3() {
        let directory = format!("{S3_NOTES}/axum_listing/");
        let paths: Vec<String> = ["one.txt", "two.txt", "three.txt"]
            .iter()
            .map(|file_name| format!("{directory}{file_name}"))
            .collect();
        let _cleanup: Vec<DeleteOnDrop> = paths.iter().map(|path| DeleteOnDrop::new(path)).collect();
        for path in &paths {
            let saved = send(save_note_request(path, HELLO_WORLD_BASE64)).await;
            assert_eq!(saved.status(), StatusCode::NO_CONTENT);
        }

        let body = format!(r#"{{"uri":"{directory}"}}"#);
        let response = send(json_request("POST", "/files/list", &body)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let listing = body_string(response).await;
        let entries: Vec<serde_json::Value> = serde_json::from_str(&listing).unwrap();
        assert_eq!(entries.len(), paths.len());
        for path in &paths {
            let listed = entries.iter().any(|entry| entry["uri"] == path.as_str());
            assert!(listed, "{path} not in {entries:?}");
        }
    }

    #[tokio::test]
    async fn test_list_directory_rejects_local_uri() {
        // Hiding the local option in the UI is cosmetic; this is the check that
        // stops a direct call enumerating the server's filesystem.
        let req = json_request("POST", "/files/list", r#"{"uri":"file:///etc"}"#);
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_directory_rejects_scheme_less_path() {
        // `StoreService` treats a scheme-less path as local, so it must not slip through.
        let req = json_request("POST", "/files/list", r#"{"uri":"/etc/passwd"}"#);
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_directory_rejects_invalid_payload() {
        let req = json_request("POST", "/files/list", r#"{"wrong":"field"}"#);
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_buckets_rejects_unknown_backend() {
        let req = Request::builder()
            .uri("/files/buckets/nonsense")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_buckets_rejects_local_backend() {
        let req = Request::builder()
            .uri("/files/buckets/local")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_file_formats() {
        let req = Request::builder()
            .uri("/files/formats")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = body_string(response).await;
        assert!(body.contains("\"format\":\"BAM\""));
        assert!(body.contains("\"indexExtension\":\".bai\""));
        // BigWig carries an embedded index.
        assert!(body.contains("\"indexExtension\":null"));
    }

    #[tokio::test]
    async fn test_cytobands_route_matches_client_path() {
        // HttpService calls `cytobands/{genome}/{chromosome}`; the route used to
        // be mounted under /genes and never matched.
        let req = Request::builder()
            .uri("/cytobands/grch38/chr1")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = body_string(response).await;
        assert!(body.starts_with("[{\"chromosome\":\"chr1\""), "{body}");
    }

    #[tokio::test]
    async fn test_gene_symbols_route_returns_genes() {
        let req = Request::builder()
            .uri("/genes/symbols/grch38/refseq/")
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = body_string(response).await;
        assert!(body.contains("\"symbol\":\"BRCA2\""));
    }

    const S3_NOTES: &str = "s3://com.gmail.docarw/test_data/notes";
    const HELLO_WORLD_BASE64: &str = "SGVsbG8gd29ybGQ=";

    fn read_note_request(path: &str) -> Request<Body> {
        let body = format!(r#"{{"path":"{path}"}}"#);
        json_request("POST", "/notes/read", &body)
    }

    fn save_note_request(path: &str, data: &str) -> Request<Body> {
        let body = format!(r#"{{"path":"{path}","data":"{data}"}}"#);
        json_request("POST", "/notes/save", &body)
    }

    #[tokio::test]
    async fn test_404_is_json() {
        let response = send(get_request("/nonexistent")).await;
        assert_json_error(response, StatusCode::NOT_FOUND).await;
    }

    #[tokio::test]
    async fn test_wrong_method_is_json_404() {
        let response = send(get_request("/search")).await;
        let message = assert_json_error(response, StatusCode::NOT_FOUND).await;
        assert_eq!(message, "Not found: GET /search");
    }

    #[tokio::test]
    async fn test_404_message() {
        let response = send(json_request("POST", "/anything", "{}")).await;
        let message = assert_json_error(response, StatusCode::NOT_FOUND).await;
        assert_eq!(message, "Not found: POST /anything");
    }

    #[tokio::test]
    async fn test_list_directory_rejects_control_character_file_uri() {
        let request = json_request("POST", "/files/list", r#"{"uri":"fi\tle:///etc"}"#);
        let response = send(request).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_read_control_character_local_note() {
        let request = json_request(
            "POST",
            "/notes/read",
            r#"{"path":"fi\tle:///home/user/analysis/notes/today.txt"}"#,
        );
        let response = send(request).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_unknown_post_is_json_404() {
        let response = send(json_request("POST", "/anything", "{}")).await;
        assert_json_error(response, StatusCode::NOT_FOUND).await;
    }

    #[tokio::test]
    async fn test_removed_recursive_listing() {
        let request = json_request("POST", "/files", r#""s3://com.gmail.docarw/test_data/""#);
        let response = send(request).await;
        assert_json_error(response, StatusCode::NOT_FOUND).await;
    }

    #[tokio::test]
    async fn test_search_invalid_json_is_json_error() {
        let response = send(json_request("POST", "/search", "not json")).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_search_empty_coordinates_error_message() {
        let request = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": []}"#,
        );
        let response = send(request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_string(response).await;
        assert_eq!(body, r#"{"error":"No coordinates"}"#);
    }

    #[tokio::test]
    async fn test_search_coordinates_must_be_a_list() {
        let request = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": "chr1:1-1000"}"#,
        );
        let response = send(request).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_search_unknown_output_format() {
        let request = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ["chr1:1-10"], "output_format": "nonsense"}"#,
        );
        let response = send(request).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_search_uses_index_path_instead_of_inferred_index() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz.tbi");
        let inferred = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000"]}"#,
        );
        let response = send(inferred).await;
        assert_eq!(response.status(), StatusCode::OK);

        let request = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000"], "index_path": "s3://com.gmail.docarw/test_data/no_such_index.tbi"}"#,
        );
        let response = send(request).await;
        let message = assert_json_error(response, StatusCode::INTERNAL_SERVER_ERROR).await;
        assert!(message.contains("no_such_index.tbi"), "{message}");
    }

    #[tokio::test]
    async fn test_search_bigwig_chr12() {
        let request = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/density.bw", "coordinates": ["chr12"], "output_format": "bigwig"}"#,
        );
        let response = send(request).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains(r#""lines":[["chr12\t"#), "{body}");
    }

    #[tokio::test]
    async fn test_search_uses_output_format_instead_of_extension() {
        let _guard = IndexGuard::new("s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz.tbi");
        let inferred = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000000"]}"#,
        );
        let response = send(inferred).await;
        assert_eq!(response.status(), StatusCode::OK);

        let request = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz", "coordinates": ["chr1:1-100000000"], "output_format": "bedgraph"}"#,
        );
        let response = send(request).await;
        assert_json_error(response, StatusCode::INTERNAL_SERVER_ERROR).await;
    }

    #[tokio::test]
    async fn test_gene_symbols_trailing_slash_optional() {
        let with_slash = send(get_request("/genes/symbols/grch38/refseq/")).await;
        assert_eq!(with_slash.status(), StatusCode::OK);
        let with_slash_body = body_string(with_slash).await;

        let without_slash = send(get_request("/genes/symbols/grch38/refseq")).await;
        assert_eq!(without_slash.status(), StatusCode::OK);
        let without_slash_body = body_string(without_slash).await;

        assert_eq!(with_slash_body, without_slash_body);
    }

    #[tokio::test]
    async fn test_cytobands_trailing_slash() {
        let response = send(get_request("/cytobands/grch38/chr1/")).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        let bands: serde_json::Value = serde_json::from_str(&body).unwrap();
        let bands = bands.as_array().unwrap();
        assert!(!bands.is_empty());
        assert!(bands.iter().all(|band| band["chromosome"] == "chr1"), "{body}");
    }

    #[tokio::test]
    async fn test_cytobands_empty_genome_is_routed() {
        let response = send(get_request("/cytobands//chr1")).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cytobands_unknown_chromosome_is_empty() {
        let response = send(get_request("/cytobands/grch38/chrNotReal")).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_string(response).await, "[]");
    }

    #[tokio::test]
    async fn test_save_then_read_note() {
        let path = format!("{S3_NOTES}/axum_round_trip.txt");
        let _cleanup = DeleteOnDrop::new(&path);
        let saved = send(save_note_request(&path, HELLO_WORLD_BASE64)).await;
        assert_eq!(saved.status(), StatusCode::NO_CONTENT);
        assert!(body_string(saved).await.is_empty());

        let read = send(read_note_request(&path)).await;
        assert_eq!(read.status(), StatusCode::OK);
        assert_eq!(body_string(read).await, r#"{"data":"SGVsbG8gd29ybGQ="}"#);
    }

    #[tokio::test]
    async fn test_read_missing_note() {
        let path = format!("{S3_NOTES}/axum_does_not_exist.txt");
        let response = send(read_note_request(&path)).await;
        assert_json_error(response, StatusCode::NOT_FOUND).await;
    }

    #[tokio::test]
    async fn test_read_note_outside_notes_directory() {
        let response = send(read_note_request("s3://com.gmail.docarw/test.txt")).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_read_local_note() {
        let response = send(read_note_request("file:///home/user/analysis/notes/today.txt")).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_save_scheme_less_note() {
        let response = send(save_note_request("/analysis/notes/today.txt", HELLO_WORLD_BASE64)).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_save_note_outside_notes_directory() {
        let path = "s3://com.gmail.docarw/test_data/axum_never_written.txt";
        let _cleanup = DeleteOnDrop::new(path);
        let response = send(save_note_request(path, HELLO_WORLD_BASE64)).await;
        assert_json_error(response, StatusCode::BAD_REQUEST).await;
        assert_eq!(http_parity::check_not_written(path).await, Ok(()));
    }

    #[tokio::test]
    async fn test_save_note_bad_base64() {
        let path = format!("{S3_NOTES}/axum_never_written_bad_data.txt");
        let _cleanup = DeleteOnDrop::new(&path);
        let saved = send(save_note_request(&path, "not base64!")).await;
        assert_json_error(saved, StatusCode::BAD_REQUEST).await;

        let read = send(read_note_request(&path)).await;
        assert_eq!(read.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_read_note_malformed_body() {
        let not_json = send(json_request("POST", "/notes/read", "not json")).await;
        assert_json_error(not_json, StatusCode::BAD_REQUEST).await;

        let missing_path = send(json_request("POST", "/notes/read", r#"{"wrong":"field"}"#)).await;
        assert_json_error(missing_path, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn test_save_note_malformed_body() {
        let not_json = send(json_request("POST", "/notes/save", "not json")).await;
        assert_json_error(not_json, StatusCode::BAD_REQUEST).await;

        let no_data_path = format!("{S3_NOTES}/axum_never_written_no_data.txt");
        let _cleanup = DeleteOnDrop::new(&no_data_path);
        let path = format!(r#"{{"path":"{no_data_path}"}}"#);
        let missing_data = send(json_request("POST", "/notes/save", &path)).await;
        assert_json_error(missing_data, StatusCode::BAD_REQUEST).await;
        assert_eq!(http_parity::check_not_written(&no_data_path).await, Ok(()));
    }

    mod http_parity {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/http_parity.rs"));
    }

    use http_parity::{DeleteOnDrop, ObservedResponse, ParityRequest};
    use tokio::task::JoinSet;

    fn parity_request(request: &ParityRequest) -> Request<Body> {
        let mut builder = Request::builder().method(request.method).uri(&request.path);
        if let Some(content_type) = request.content_type {
            builder = builder.header(header::CONTENT_TYPE, content_type);
        }
        let body = Body::from(request.body.clone());
        builder.body(body).unwrap()
    }

    async fn observe(response: axum::response::Response) -> ObservedResponse {
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|value| value.to_str().unwrap().to_string());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        ObservedResponse {
            status,
            content_type,
            body: bytes.to_vec(),
        }
    }

    async fn send_parity(request: &ParityRequest) -> ObservedResponse {
        let http_request = parity_request(request);
        let response = send(http_request).await;
        observe(response).await
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
        cases.push(http_parity::head_case(true));
        let mut failures = Vec::new();
        for case in cases {
            let response = send_parity(&case.request).await;
            if let Err(failure) = http_parity::check_case(&case, &response) {
                failures.push(failure);
            }
        }
        failures.extend(http_parity::rejected_saves_written().await);
        http_parity::assert_no_failures("seqa_axum", &failures);
    }

    #[tokio::test]
    async fn parity_preflight_from_dev_ui_is_allowed() {
        let dev_origin = "http://localhost:5173";
        let request = Request::builder()
            .method("OPTIONS")
            .uri("/search")
            .header(header::ORIGIN, dev_origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .body(Body::empty())
            .unwrap();
        let response = send(request).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], dev_origin);
    }

    #[tokio::test]
    async fn parity_binary_note_saved_over_http_is_byte_exact() {
        let path = http_parity::note_path("axum_parity_binary.bin");
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
        let path = http_parity::note_path("axum_parity_written_by_core.bin");
        let _cleanup = DeleteOnDrop::new(&path);
        http_parity::save_binary_note_with_core(&path).await;
        let request = http_parity::read_note_request(&path);
        let response = send_parity(&request).await;
        assert_eq!(http_parity::check_binary_note_read(&response), Ok(()));
    }

    #[tokio::test]
    async fn parity_form_encoded_save_is_not_written() {
        let path = http_parity::note_path("axum_parity_never_written_form.txt");
        let _cleanup = DeleteOnDrop::new(&path);
        let request = http_parity::form_save_note_request(&path);
        let response = send_parity(&request).await;
        assert_eq!(response.status, 400);
        let written = http_parity::check_not_written(&path).await;
        assert_eq!(written, Ok(()));
    }

    #[tokio::test]
    async fn parity_concurrent_note_saves_on_one_service() {
        let service = app();
        let paths = http_parity::concurrent_note_paths("axum", 8);
        let _cleanup: Vec<DeleteOnDrop> = paths.iter().map(|path| DeleteOnDrop::new(path)).collect();
        let mut saves = JoinSet::new();
        for path in &paths {
            let request = http_parity::save_note_request(path, http_parity::EVERY_BYTE_BASE64);
            let http_request = parity_request(&request);
            let service = service.clone();
            saves.spawn(async move { service.oneshot(http_request).await.unwrap().status() });
        }
        let statuses = saves.join_all().await;
        assert!(statuses.iter().all(|status| *status == StatusCode::NO_CONTENT), "{statuses:?}");
        for path in &paths {
            let data = http_parity::read_note_with_core(path).await;
            assert_eq!(data, http_parity::EVERY_BYTE_BASE64, "{path}");
        }
    }

    #[tokio::test]
    async fn parity_cached_search_matches_first_search() {
        let _guard = IndexGuard::new(http_parity::VCF_INDEX);
        let service = app();
        let body = format!(
            r#"{{"path": "{}", "coordinates": ["chr1:116549-116549", "chr1:1-2"], "with_header": true}}"#,
            http_parity::VCF
        );
        let first_request = json_request("POST", "/search", &body);
        let first = service.clone().oneshot(first_request).await.unwrap();
        let first_body = body_string(first).await;

        let mut searches = JoinSet::new();
        for _ in 0..4 {
            let request = json_request("POST", "/search", &body);
            let service = service.clone();
            searches.spawn(async move {
                let response = service.oneshot(request).await.unwrap();
                body_string(response).await
            });
        }
        let cached_bodies = searches.join_all().await;
        assert!(cached_bodies.iter().all(|cached| *cached == first_body));
        assert!(first_body.contains("#CHROM"), "{first_body}");
    }
}

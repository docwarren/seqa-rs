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
use crate::search::models::SearchRequest;
use axum::{
    Json, Router,
    extract::{Path, Request, State, rejection::JsonRejection},
    http::{Method, StatusCode, Uri, header},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use seqa_core::api::output_format::OutputFormat;
use seqa_core::api::search::search_features;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::models::cytoband::Cytoband;
use seqa_core::models::gene_coordinate::GeneCoordinate;
use seqa_core::sqlite::{self, genes};
use seqa_core::stores::buckets::{CloudBackend, list_buckets as list_cloud_buckets};
use seqa_core::stores::{DirectoryEntry, StoreService};
use seqa_core::utils::{FileFormat, supported_formats};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tower_http::cors::CorsLayer;

pub mod cache;
pub mod search;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<StoreService>,
    pub cache: AppCache,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub last_modified: String,
    pub size: u64,
}

/// Body of `POST /files/list`.
///
/// A POST rather than a path parameter because URIs contain `://` and slashes,
/// which a path segment mangles.
#[derive(Deserialize)]
pub struct ListDirectoryRequest {
    pub uri: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Internal Server Error: {0}")]
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

async fn index() -> &'static str {
    "Hello world"
}

async fn feature_search(
    State(state): State<AppState>,
    payload: Result<Json<SearchRequest>, JsonRejection>,
) -> Result<String, ApiError> {
    let Json(request) =
        payload.map_err(|e| ApiError::BadRequest(format!("Invalid JSON payload: {}", e)))?;

    if !request.path.ends_with(".bam")
        && !request.path.ends_with(".vcf.gz")
        && !request.path.ends_with(".bed.gz")
        && !request.path.ends_with(".bedgraph.gz")
        && !request.path.ends_with(".gff.gz")
        && !request.path.ends_with(".fasta")
        && !request.path.ends_with(".fa")
        && !request.path.ends_with(".bigwig")
        && !request.path.ends_with(".bb")
        && !request.path.ends_with(".bigbed")
        && !request.path.ends_with(".bw")
    {
        return Err(ApiError::BadRequest("Unsupported file type".to_string()));
    }
    if request.coordinates.is_empty() {
        return Err(ApiError::BadRequest("Empty coordinate string".to_string()));
    }

    let mut search_options = SearchOptions::new(&request.path, &vec![request.coordinates])
        .map_err(|e| ApiError::BadRequest(format!("Invalid coordinates: {}", e)))?;

    populate_from_cache(&state.cache, &mut search_options).await;
    let result = search_features(&state.store, &search_options)
        .await
        .map_err(|e| ApiError::InternalError(format!("Search Failed: {:?}", e.to_string())))?;
    write_back_to_cache(&state.cache, &search_options, &result).await;

    Ok(result.lines.into_iter().collect::<Vec<String>>().join("\n"))
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
    if let Some(header) = &result.bam_header {
        cache
            .bam_header
            .insert(options.file_path.clone(), header.clone())
            .await;
    }
    if let Some(tabix) = &result.tabix_index {
        cache
            .tabix
            .insert(options.index_path.clone(), tabix.clone())
            .await;
    }
    if let Some(header) = &result.tabix_header {
        cache
            .tabix_header
            .insert(options.file_path.clone(), header.clone())
            .await;
    }
    if let Some(fai) = &result.fasta_index {
        cache
            .fai
            .insert(options.index_path.clone(), fai.clone())
            .await;
    }
}

async fn list_dir(
    State(state): State<AppState>,
    payload: Result<Json<String>, JsonRejection>,
) -> Result<Json<Vec<FileEntry>>, ApiError> {
    let Json(dir) = payload.map_err(|e| ApiError::BadRequest(e.body_text()))?;

    let objects = state
        .store
        .list_objects(&dir)
        .await
        .map_err(|e| ApiError::NotFound(format!("Failed to list {}: {}", dir, e)))?;

    let entries: Vec<FileEntry> = objects
        .into_iter()
        .map(|meta| FileEntry {
            path: meta.location.to_string(),
            last_modified: meta.last_modified.to_rfc3339(),
            size: meta.size,
        })
        .collect();

    Ok(Json(entries))
}

async fn get_gene_symbols(
    Path(genome): Path<String>,
) -> Result<Json<Vec<GeneCoordinate>>, ApiError> {
    let connection = sqlite::connect_genes(&genome)
        .map_err(|e| ApiError::InternalError(format!("Error connecting to DB: {}", e)))?;
    let symbols = genes::get_gene_symbols(&connection)
        .map_err(|e| ApiError::InternalError(format!("Error fetching symbols from DB: {}", e)))?;
    Ok(Json(symbols))
}

async fn get_cytobands(
    Path((genome, chromosome)): Path<(String, String)>,
) -> Result<Json<Vec<Cytoband>>, ApiError> {
    let connection = sqlite::connect_cytobands(&genome)
        .map_err(|e| ApiError::InternalError(format!("Error connecting to DB: {}", e)))?;
    let cytobands = genes::get_cytobands(&connection, &chromosome)
        .map_err(|e| ApiError::InternalError(format!("Error fetching cytobands from DB: {}", e)))?;
    Ok(Json(cytobands))
}

/// Lists one level of a directory or bucket prefix.
///
/// Local (`file://`) URIs are rejected: on a server that would expose the
/// *server's* filesystem to any client CORS permits, which combined with
/// `POST /search` reading arbitrary URIs is file disclosure. The frontend also
/// hides the local backend on web, but that is a UX choice — this check is what
/// actually enforces it, since the endpoint is reachable directly.
async fn list_directory(
    State(state): State<AppState>,
    payload: Result<Json<ListDirectoryRequest>, JsonRejection>,
) -> Result<Json<Vec<DirectoryEntry>>, ApiError> {
    let Json(request) =
        payload.map_err(|e| ApiError::BadRequest(format!("Invalid JSON payload: {}", e)))?;

    if is_local_uri(&request.uri) {
        return Err(ApiError::BadRequest(
            "Local file browsing is not available over HTTP".to_string(),
        ));
    }

    let entries = state
        .store
        .list_directory(&request.uri)
        .await
        .map_err(|e| ApiError::InternalError(format!("Error listing {}: {}", request.uri, e)))?;

    Ok(Json(entries))
}

/// True for URIs that would resolve to the server's own filesystem.
fn is_local_uri(uri: &str) -> bool {
    let lower = uri.trim().to_ascii_lowercase();
    // A scheme-less path is treated as local by `StoreService`, so reject it too.
    lower.starts_with("file:") || !lower.contains("://")
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

async fn not_found_fallback(req: Request) -> (StatusCode, Html<String>) {
    let uri: &Uri = req.uri();
    (
        StatusCode::NOT_FOUND,
        Html(format!("<p>404: Not Found - {}</p>", uri)),
    )
}

pub fn app() -> Router {
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

    Router::new()
        .route("/", get(index))
        .route("/search", post(feature_search))
        .route("/files", post(list_dir))
        .route("/files/list", post(list_directory))
        .route("/files/buckets/{backend}", get(list_buckets))
        .route("/files/formats", get(get_file_formats))
        .route("/genes/symbols/{genome}", get(get_gene_symbols))
        .route("/cytobands/{genome}/{chromosome}", get(get_cytobands))
        .fallback(not_found_fallback)
        .layer(cors)
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8000";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    println!("seqa_axum listening on http://{}", addr);
    axum::serve(listener, app()).await.expect("server error");
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn json_request(method: &str, uri: &str, body: &'static str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    async fn body_string(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_index() {
        let response = app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_string(response).await, "Hello world");
    }

    #[tokio::test]
    async fn test_search_unsupported_file_type() {
        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://bucket/file.txt", "coordinates": "chr1:1-1000"}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_search_missing_coordinates() {
        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ""}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_search_invalid_json() {
        let req = json_request("POST", "/search", r#"not json"#);
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_404() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_search_vcf() {
        let req = json_request(
            "POST",
            "/search",
            r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": "chr1:116549-116549"}"#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("chr1\t116549"));
        seqa_core::indexes::index_cache::delete_local_index(
            "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi",
        );
    }

    #[tokio::test]
    async fn test_list_files_s3() {
        let req = json_request("POST", "/files", r#""s3://com.gmail.docarw/test_data/""#);
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("NA12877"));
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
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }
}

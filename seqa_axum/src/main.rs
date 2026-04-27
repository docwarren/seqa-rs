use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Request, State, rejection::JsonRejection},
    http::{HeaderValue, Method, StatusCode, header, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use seqa_core::api::output_format::OutputFormat;
use seqa_core::api::search::SearchFeaturesError;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::models::cytoband::Cytoband;
use seqa_core::models::gene_coordinate::GeneCoordinate;
use seqa_core::sqlite::genes::{self, GeneError};
use seqa_core::stores::StoreService;
use seqa_core::utils::UtilError;
use thiserror::Error;
use tower_http::cors::CorsLayer;

use crate::cache::AppCache;
use crate::search::models::SearchRequest;

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

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Internal server error")]
    InternalServerError,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Search error: {0}")]
    SearchError(#[from] SearchFeaturesError),

    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("Invalid search request: {0}")]
    UtilError(#[from] UtilError),

    #[error("Storage error: {0}")]
    StoreError(String),

    #[error("Gene lookup error: {0}")]
    GeneError(#[from] GeneError),

    #[error("Patient not found: {0}")]
    PatientNotFound(String),

    #[error("User not found: {0}")]
    UserNotFound(String),
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::SearchError(_) => StatusCode::BAD_REQUEST,
            ApiError::SqliteError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::UtilError(_) => StatusCode::BAD_REQUEST,
            ApiError::StoreError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::GeneError(_) => StatusCode::NOT_FOUND,
            ApiError::PatientNotFound(_) => StatusCode::NOT_FOUND,
            ApiError::UserNotFound(_) => StatusCode::NOT_FOUND,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorResponse {
            error: self.to_string(),
            code: status.as_u16(),
        };
        (status, Json(body)).into_response()
    }
}

async fn index() -> &'static str {
    "Hello world"
}

async fn search_features(
    State(state): State<AppState>,
    payload: Result<Json<SearchRequest>, JsonRejection>,
) -> Result<String, ApiError> {
    let Json(request) = payload.map_err(|e| ApiError::BadRequest(e.body_text()))?;

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
        return Err(ApiError::BadRequest("Unsupported file type".into()));
    }
    if request.coordinates.is_empty() {
        return Err(ApiError::BadRequest("Missing coordinates".into()));
    }

    let mut search_options = SearchOptions::new(&request.path, &request.coordinates);
    populate_from_cache(&state.cache, &mut search_options).await;
    let result = state.store.search_features(&search_options).await?;
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
        cache.bai.insert(options.index_path.clone(), bai.clone()).await;
    }
    if let Some(header) = &result.bam_header {
        cache.bam_header.insert(options.file_path.clone(), header.clone()).await;
    }
    if let Some(tabix) = &result.tabix_index {
        cache.tabix.insert(options.index_path.clone(), tabix.clone()).await;
    }
    if let Some(header) = &result.tabix_header {
        cache.tabix_header.insert(options.file_path.clone(), header.clone()).await;
    }
    if let Some(fai) = &result.fasta_index {
        cache.fai.insert(options.index_path.clone(), fai.clone()).await;
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
        .map_err(|e| ApiError::StoreError(format!("Failed to list {}: {}", dir, e)))?;

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

fn normalize_genome(genome: &str) -> &'static str {
    match genome.to_ascii_lowercase().as_str() {
        "grch37" | "hg19" => "grch37",
        _ => "grch38",
    }
}

async fn get_gene_symbols(Path(genome): Path<String>) -> Result<Json<Vec<String>>, ApiError> {
    let url = format!("./data/{}-genes.db", normalize_genome(&genome));
    let connection = genes::establish_connection(url.clone())
        .map_err(|e| ApiError::DatabaseError(format!("Failed to open {}: {}", url, e)))?;
    let symbols = genes::get_gene_symbols(&connection)?;
    Ok(Json(symbols))
}

async fn get_coordinates(
    Path((genome, gene)): Path<(String, String)>,
) -> Result<Json<GeneCoordinate>, ApiError> {
    let url = format!("./data/{}-genes.db", normalize_genome(&genome));
    let connection = genes::establish_connection(url.clone())
        .map_err(|e| ApiError::DatabaseError(format!("Failed to open {}: {}", url, e)))?;
    let coord = genes::get_gene_coordinates(&connection, &gene)?;
    Ok(Json(coord))
}

async fn get_cytobands(
    Path((genome, chromosome)): Path<(String, String)>,
) -> Result<Json<Vec<Cytoband>>, ApiError> {
    let url = format!("./data/{}-cytobands.db", normalize_genome(&genome));
    let connection = genes::establish_connection(url.clone())
        .map_err(|e| ApiError::DatabaseError(format!("Failed to open {}: {}", url, e)))?;
    let cytobands = genes::get_cytobands(&connection, &chromosome)?;
    Ok(Json(cytobands))
}

async fn not_found_fallback(req: Request) -> (StatusCode, Html<String>) {
    let uri: &Uri = req.uri();
    (
        StatusCode::NOT_FOUND,
        Html(format!("<p>404: Not Found - {}</p>", uri)),
    )
}

pub fn app() -> Router {
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true);

    let state = AppState {
        store: Arc::new(StoreService::new()),
        cache: AppCache::new(),
    };

    Router::new()
        .route("/", get(index))
        .route("/search", post(search_features))
        .route("/files", post(list_dir))
        .route("/genes/symbols/{genome}", get(get_gene_symbols))
        .route("/genes/coordinates/{genome}/{gene}", get(get_coordinates))
        .route("/genes/cytobands/{genome}/{chromosome}", get(get_cytobands))
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
        let req = json_request(
            "POST",
            "/files",
            r#""s3://com.gmail.docarw/test_data/""#,
        );
        let response = app().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_string(response).await;
        assert!(body.contains("NA12877"));
    }
}

#[macro_use] extern crate rocket;

use seqa_core::sqlite::genes::{self, GeneError};
use seqa_core::stores::StoreService;

use rocket::response::content;
use rocket::serde::json::Json;
use rocket::{ Request, catch, catchers, routes };
use rocket::http::{ContentType, Status};
use rocket::serde::json::serde_json;
use rocket_cors::{ CorsOptions, AllowedHeaders, AllowedOrigins };
use rocket_cors::Method;
use serde::Serialize;
use seqa_core::utils::{get_search_options, UtilError};

use std::collections::HashSet;
use std::io::Cursor;
use thiserror::Error;

use seqa_core::models::FileSearchRequest;
use seqa_core::models::gene_coordinate::GeneCoordinate;
use seqa_core::services::search::{ SearchError, SearchService };
use crate::search::models::SearchRequest;

pub mod search;

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
    SearchError(#[from] SearchError),

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
    fn status_code(&self) -> Status {
        match self {
            ApiError::InternalServerError => Status::InternalServerError,
            ApiError::NotFound(_) => Status::NotFound,
            ApiError::BadRequest(_) => Status::BadRequest,
            ApiError::DatabaseError(_) => Status::InternalServerError,
            ApiError::SearchError(_) => Status::BadRequest,
            ApiError::SqliteError(_) => Status::InternalServerError,
            ApiError::UtilError(_) => Status::BadRequest,
            ApiError::StoreError(_) => Status::InternalServerError,
            ApiError::GeneError(_) => Status::NotFound,
            ApiError::PatientNotFound(_) => Status::NotFound,
            ApiError::UserNotFound(_) => Status::NotFound,
        }
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for ApiError {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let status = self.status_code();
        let error_response = ErrorResponse {
            error: self.to_string(),
            code: status.code,
        };

        let body = serde_json::to_string(&error_response)
            .unwrap_or_else(|_| r#"{"error":"Internal server error","code":500}"#.to_string());

        rocket::Response::build()
            .status(status)
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello world"
}

#[post("/", data = "<search_request>")]
async fn search_features(search_request: Json<SearchRequest>) -> Result<String, ApiError> {
    let request = search_request.into_inner();
    if !request.path.ends_with(".bam") &&
       !request.path.ends_with(".vcf.gz") &&
       !request.path.ends_with(".bed.gz") &&
       !request.path.ends_with(".bedgraph.gz") &&
       !request.path.ends_with(".gff.gz") &&
       !request.path.ends_with(".fasta") &&
       !request.path.ends_with(".fa") &&
       !request.path.ends_with(".bigwig") &&
       !request.path.ends_with(".bb") &&
       !request.path.ends_with(".bigbed") &&
       !request.path.ends_with(".bw") {
        return Err(ApiError::BadRequest("Unsupported file type".into()));
    }
    if request.coordinates.is_empty() {
        return Err(ApiError::BadRequest("Missing coordinates".into()));
    }
    let search_request = FileSearchRequest::new(request.path, request.coordinates);
    let search_options = get_search_options(search_request)?;
    let result = SearchService::search_features(&search_options).await?;

    Ok(result.lines.into_iter().collect::<Vec<String>>().join("\n"))
}

#[post("/", data = "<dir_path>")]
async fn list_dir(dir_path: Json<String>) -> Result<Json<Vec<FileEntry>>, ApiError> {
    let dir = dir_path.into_inner();

    let service = StoreService::from_uri(&dir)
        .map_err(|e| ApiError::StoreError(format!("Invalid path {}: {}", dir, e)))?;

    let objects = service.list_objects(&dir)
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

#[get("/<genome>")]
async fn get_gene_symbols(genome: &str) -> Result<Json<Vec<String>>, ApiError> {
    let url = format!("./data/{}-genes.db", genome.to_lowercase());
    let connection = genes::establish_connection(url.clone())
        .map_err(|e| ApiError::DatabaseError(format!("Failed to open {}: {}", url, e)))?;
    let symbols = genes::get_gene_symbols(&connection)?;
    Ok(Json(symbols))
}

#[get("/<genome>/<gene>")]
async fn get_coordinates(gene: &str, genome: &str) -> Result<Json<GeneCoordinate>, ApiError> {
    let url = format!("./data/{}-genes.db", genome.to_lowercase());
    let connection = genes::establish_connection(url.clone())
        .map_err(|e| ApiError::DatabaseError(format!("Failed to open {}: {}", url, e)))?;
    let coord = genes::get_gene_coordinates(&connection, gene)?;
    Ok(Json(coord))
}

#[catch(404)]
fn not_found_catcher(req: &Request) -> content::RawHtml<String> {
    content::RawHtml(format!("<p>404: Not Found - {}</p>", req.uri()))
}

pub fn rocket() -> rocket::Rocket<rocket::Build> {
    let allowed_urls = AllowedOrigins::some_regex(&["http://localhost:5173".to_string()]);

    let methods = [
        Method::from(rocket::http::Method::Get),
        Method::from(rocket::http::Method::Post),
    ];
    let allowed_methods: HashSet<Method> = HashSet::from(methods);
    let allowed_headers = AllowedHeaders::some(&["Authorization", "Content-Type"]);

    let cors = CorsOptions::default()
        .allowed_origins(allowed_urls)
        .allowed_methods(allowed_methods)
        .allowed_headers(allowed_headers)
        .allow_credentials(true)
        .to_cors()
        .expect("Failed to create CORS options");

    rocket::build()
        .attach(cors)
        .mount("/", routes![index])
        .mount("/genes/symbols", routes![get_gene_symbols])
        .mount("/genes/coordinates", routes![get_coordinates])
        .mount("/search", routes![search_features])
        .mount("/files", routes![list_dir])
        .register("/", catchers![not_found_catcher])
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    rocket().launch().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;

    async fn client() -> Client {
        Client::tracked(rocket()).await.expect("valid rocket instance")
    }

    #[rocket::async_test]
    async fn test_index() {
        let client = client().await;
        let response = client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().await.unwrap(), "Hello world");
    }

    #[rocket::async_test]
    async fn test_search_unsupported_file_type() {
        let client = client().await;
        let response = client.post("/search")
            .header(ContentType::JSON)
            .body(r#"{"path": "s3://bucket/file.txt", "coordinates": "chr1:1-1000"}"#)
            .dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn test_search_missing_coordinates() {
        let client = client().await;
        let response = client.post("/search")
            .header(ContentType::JSON)
            .body(r#"{"path": "s3://bucket/file.vcf.gz", "coordinates": ""}"#)
            .dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn test_search_invalid_json() {
        let client = client().await;
        let response = client.post("/search")
            .header(ContentType::JSON)
            .body(r#"not json"#)
            .dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn test_404() {
        let client = client().await;
        let response = client.get("/nonexistent").dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn test_search_vcf() {
        let client = client().await;
        let response = client.post("/search")
            .header(ContentType::JSON)
            .body(r#"{"path": "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz", "coordinates": "chr1:116549-116549"}"#)
            .dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.unwrap();
        assert!(body.contains("chr1\t116549"));
        seqa_core::indexes::index_cache::delete_local_index(
            "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi"
        );
    }

    #[rocket::async_test]
    async fn test_list_files_s3() {
        let client = client().await;
        let response = client.post("/files")
            .header(ContentType::JSON)
            .body(r#""s3://com.gmail.docarw/test_data/""#)
            .dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.unwrap();
        assert!(body.contains("NA12877"));
    }
}

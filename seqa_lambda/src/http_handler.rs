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

use lambda_http::http::Method;
use lambda_http::{Body, Error, Request, RequestPayloadExt, Response};
use ::serde::{ Serialize, Deserialize };
use seqa_core::api::search_options::{CigarFormat, SearchOptions};
use seqa_core::sqlite::{self, genes};
use seqa_core::stores::StoreService;

#[derive(Serialize, Deserialize)]
pub struct SearchRequest {
    pub path: String,
    pub coordinates: String,
    pub with_header: Option<bool>,
    pub only_header: Option<bool>,
    pub genome: Option<String>,
    pub index_path: Option<String>,
    pub output_format: Option<String>,
}

pub(crate) async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let method = event.method().clone();
    let path = event.uri().path().to_string();
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();

    match (&method, segments.as_slice()) {
        (&Method::GET, ["genes", "symbols", genome]) => {
            gene_symbols(genome)
        }
        (&Method::GET, ["genes", "coordinates", genome, gene]) => {
            gene_coordinates(genome, gene)
        }
        (&Method::GET, ["genes", "cytobands", genome, chromosome]) => {
            cytobands(genome, chromosome)
        }
        (&Method::POST, _) => search_handler(event).await,
        _ => Ok(Response::builder()
            .status(404)
            .body(format!("Not found: {} {}", method, path).into())
            .map_err(Box::new)?),
    }
}

async fn search_handler(event: Request) -> Result<Response<Body>, Error> {
    let options: SearchOptions = match event.payload::<SearchRequest>()? {
        Some(request) => {
            let genome = request.genome.unwrap_or("hg38".to_string());
            SearchOptions::new(&request.path, &request.coordinates)
                .set_include_header(request.with_header.unwrap_or(false))
                .set_header_only(request.only_header.unwrap_or(false))
                .set_cigar_format(CigarFormat::Standard)
                .set_genome(&genome)
        }
        _ => {
            return Ok(Response::builder()
                .status(400)
                .body("Invalid request body".into())
                .map_err(Box::new)?)
        }
    };

    if let Ok(store) = StoreService::from_uri(&options.file_path) {
        let result = store.search_features(&options).await?;
        let lines = serde_json::to_string(&result.lines)?;

        return Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Body::Text(lines))
            .map_err(Box::new)?);
    }
    Ok(Response::builder()
        .status(500)
        .body("Internal Server Error".into())
        .map_err(Box::new)?)
}

fn gene_symbols(genome: &str) -> Result<Response<Body>, Error> {
    let conn = match sqlite::connect_genes(genome) {
        Ok(c) => c,
        Err(e) => return error_response(500, &format!("{}", e)),
    };
    match genes::get_gene_symbols(&conn) {
        Ok(symbols) => json_response(200, &symbols),
        Err(e) => error_response(500, &format!("{}", e)),
    }
}

fn gene_coordinates(genome: &str, gene: &str) -> Result<Response<Body>, Error> {
    let conn = match sqlite::connect_genes(genome) {
        Ok(c) => c,
        Err(e) => return error_response(500, &format!("{}", e)),
    };
    match genes::get_gene_coordinates(&conn, gene) {
        Ok(coord) => json_response(200, &coord),
        Err(e) => error_response(404, &format!("{}", e)),
    }
}

fn cytobands(genome: &str, chromosome: &str) -> Result<Response<Body>, Error> {
    let conn = match sqlite::connect_cytobands(genome) {
        Ok(c) => c,
        Err(e) => return error_response(500, &format!("{}", e)),
    };
    match genes::get_cytobands(&conn, chromosome) {
        Ok(bands) => json_response(200, &bands),
        Err(e) => error_response(500, &format!("{}", e)),
    }
}

fn json_response<T: Serialize>(status: u16, value: &T) -> Result<Response<Body>, Error> {
    let body = serde_json::to_string(value)?;
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::Text(body))
        .map_err(Box::new)?)
}

fn error_response(status: u16, message: &str) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::Text(serde_json::json!({ "error": message }).to_string()))
        .map_err(Box::new)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::{Body, Request};

    fn build_request(options: &SearchOptions) -> Request {
        let payload = SearchRequest {
            path: options.file_path.clone(),
            coordinates: format!("{}:{}-{}", options.chromosome, options.begin, options.end),
            with_header: Some(options.include_header),
            only_header: Some(options.header_only),
            genome: options.genome.clone(),
            index_path: Some(options.index_path.clone()),
            output_format: Some(options.output_format.to_string()),
        };
        let body = serde_json::to_string(&payload).unwrap();
        let mut request = Request::new(Body::Text(body));
        *request.method_mut() = Method::POST;
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

    #[tokio::test]
    async fn bigwig_chr12() {
        let options = SearchOptions::new("s3://com.gmail.docarw/test_data/density.bw", "chr12")
            .set_index_path("-")
            .set_output_format("bigwig")
            .set_include_header(false);

        let response = function_handler(build_request(&options)).await.unwrap();
        assert_eq!(response.status(), 200);

        let lines: Vec<String> = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert!(!lines.is_empty(), "expected BigWig chr12 results");

        for line in &lines {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields[0], "chr12", "unexpected chromosome in line: {}", line);
            let begin: u32 = fields[1].parse().unwrap();
            let end: u32 = fields[2].parse().unwrap();
            assert!(begin < end, "expected begin < end, got {}-{}", begin, end);
        }
    }

    #[tokio::test]
    async fn bam_chr2_range() {
        let options = SearchOptions::new(
            "s3://com.gmail.docarw/test_data/NA12877.bam",
            "chr2:50000000-50000100",
        )
        .set_index_path("s3://com.gmail.docarw/test_data/NA12877.bam.bai")
        .set_output_format("bam")
        .set_include_header(false);

        let response = function_handler(build_request(&options)).await.unwrap();
        assert_eq!(response.status(), 200);

        let lines: Vec<String> = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert!(!lines.is_empty(), "expected BAM chr2 results");

        for line in &lines {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields[2], "chr2", "unexpected reference in line: {}", line);
        }
    }

    #[tokio::test]
    async fn gene_coordinates_brca2_grch38() {
        let response = function_handler(get_request("/genes/coordinates/grch38/BRCA2"))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body: serde_json::Value = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert_eq!(body["gene"], "BRCA2");
    }

    #[tokio::test]
    async fn cytobands_chr1_grch38() {
        let response = function_handler(get_request("/genes/cytobands/grch38/chr1"))
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let bands: serde_json::Value = serde_json::from_slice(&response.body().to_vec()).unwrap();
        assert!(bands.as_array().unwrap().len() > 0);
    }
}

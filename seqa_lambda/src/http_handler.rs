use lambda_http::{Body, Error, Request, RequestPayloadExt, Response};
use ::serde::{ Serialize, Deserialize };
use seqa_core::api::search_options::{CigarFormat, SearchOptions};
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

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
pub(crate) async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let options: SearchOptions =  match event.payload::<SearchRequest>()? {

        Some(request) => {
            let genome = request.genome.unwrap_or("hg38".to_string());
            let options = SearchOptions::new(&request.path, &request.coordinates)
                .set_include_header(request.with_header.unwrap_or(false))
                .set_header_only(request.only_header.unwrap_or(false))
                .set_cigar_format(CigarFormat::Standard)
                .set_genome(&genome);
            options
        },
        _ => return Ok(Response::builder().status(400).body("Invalid request body".into()).map_err(Box::new)?)
    };

    if let Ok(store) = StoreService::from_uri(&options.file_path) {
        let result = store.search_features(&options).await?;
            let lines = serde_json::to_string(&result.lines)?;

            return Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Body::Text(lines))
                .map_err(Box::new)?)
    }
    Ok(Response::builder().status(500).body("Internal Server Error".into()).map_err(Box::new)?)
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
        request
            .headers_mut()
            .insert("content-type", "application/json".parse().unwrap());
        request
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
}

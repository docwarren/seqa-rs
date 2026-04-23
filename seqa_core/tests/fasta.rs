const S3_FASTA: &str = "s3://com.gmail.docarw/test_data/grch38.fa";
const S3_FASTA_INDEX: &str = "s3://com.gmail.docarw/test_data/grch38.fa.fai";

#[tokio::test]
async fn fasta_chr1() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_FASTA, "chr1:10000-10100")
        .set_index_path(S3_FASTA_INDEX)
        .set_output_format("fasta")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = store_service.search_features(&options).await.expect("Failed to search FASTA for chr1");
    assert!(!result.lines.is_empty(), "FASTA search should return results");
}

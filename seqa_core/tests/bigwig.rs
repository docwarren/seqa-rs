#[tokio::test]
async fn s3_bigwig() {
    use seqa_core::services::search::SearchService;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new("s3://com.gmail.docarw/test_data/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_output_format("bigwig")
        .set_include_header(false);

    let result = SearchService::search_features(&options).await.expect("Failed to search BigWig for chr4");
    let begin = result.lines[0].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let last_begin = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let end = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[2].parse::<u32>().unwrap();

    assert!(result.lines.len() < 5000);
    assert!(begin < end);
    assert!(begin > 120000000);
    assert!(end > last_begin);
    assert!(last_begin < 140000000);
}

#[tokio::test]
async fn azure_bigwig() {
    use seqa_core::api::bigwig_search::bigwig_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new("az://genreblobs/genre-test-data/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_include_header(false);
    
    let result = bigwig_search(&options).await.expect("Failed to search BigWig for chr4");
    let begin = result.lines[0].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let last_begin = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let end = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[2].parse::<u32>().unwrap();

    assert!(result.lines.len() < 5000);
    assert!(begin < end);
    assert!(begin > 120000000);
    assert!(end > last_begin);
    assert!(last_begin < 140000000);
}

#[tokio::test]
async fn gc_bigwig() {
    use seqa_core::api::bigwig_search::bigwig_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new("gs://genre_test_bucket/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_include_header(false);
    
    let result = bigwig_search(&options).await.expect("Failed to search BigWig for chr4");
    let begin = result.lines[0].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let last_begin = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let end = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[2].parse::<u32>().unwrap();

    assert!(result.lines.len() < 5000);
    assert!(begin < end);
    assert!(begin > 120000000);
    assert!(end > last_begin);
    assert!(last_begin < 140000000);
}

#[tokio::test]
async fn http_bigwig() {
    use seqa_core::api::bigwig_search::bigwig_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new("https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test_data/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_include_header(false);
    
    let result = bigwig_search(&options).await.expect("Failed to search BigWig for chr4");
    let begin = result.lines[0].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let last_begin = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let end = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[2].parse::<u32>().unwrap();

    assert!(result.lines.len() < 5000);
    assert!(begin < end);
    assert!(begin > 120000000);
    assert!(end > last_begin);
    assert!(last_begin < 140000000);
}

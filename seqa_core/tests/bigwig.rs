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

#[tokio::test]
async fn s3_bigwig() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new("s3://com.gmail.docarw/test_data/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_output_format("bigwig")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = store_service.search_features(&options).await.expect("Failed to search BigWig for chr4");
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
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new("az://genreblobs/genre-test-data/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = bigwig_search(&store_service, &options).await.expect("Failed to search BigWig for chr4");
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
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new("gs://genre_test_bucket/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = bigwig_search(&store_service, &options).await.expect("Failed to search BigWig for chr4");
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
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new("https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test_data/density.bw", "chr4:120000000-140000000")
        .set_index_path("-")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = bigwig_search(&store_service, &options).await.expect("Failed to search BigWig for chr4");
    let begin = result.lines[0].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let last_begin = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[1].parse::<u32>().unwrap();
    let end = result.lines[result.lines.len() - 1].split('\t').collect::<Vec<&str>>()[2].parse::<u32>().unwrap();

    assert!(result.lines.len() < 5000);
    assert!(begin < end);
    assert!(begin > 120000000);
    assert!(end > last_begin);
    assert!(last_begin < 140000000);
}

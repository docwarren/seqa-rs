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

/// Tests for BigBed file search functionality.
/// Compares bigbed_search results against tabix_search on equivalent BED data.
use seqa_core::indexes::index_cache::IndexGuard;

const BIGBED_PATH: &str = "s3://com.soma23.data/hg38/mane.bb";
const BED_PATH: &str = "s3://com.soma23.data/hg38/mane.bed.gz";
const BED_INDEX_PATH: &str = "s3://com.soma23.data/hg38/mane.bed.gz.tbi";

#[tokio::test]
async fn bigbed_search_matches_tabix() {
    let _guard = IndexGuard::new(BED_INDEX_PATH);
    use seqa_core::api::search::search_features;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let bb_options = SearchOptions::new(BIGBED_PATH, &vec!["chr1:1000000-1300000".to_string()])
        .unwrap()
        .set_output_format("bigbed")
        .set_include_header(false);

    let store_service = StoreService::new();
    let bb_result = search_features(&store_service, &bb_options)
        .await
        .expect("Failed to search BigBed");

    let bed_options = SearchOptions::new(BED_PATH, &vec!["chr1:1000000-1300000".to_string()])
        .unwrap()
        .set_index_path(BED_INDEX_PATH)
        .set_output_format("bed")
        .set_include_header(false);

    let bed_result = search_features(&store_service, &bed_options)
        .await
        .expect("Failed to search BED");

    assert_eq!(
        bb_result.lines[0].len(),
        bed_result.lines[0].len(),
        "BigBed returned {} lines, BED returned {} lines",
        bb_result.lines[0].len(),
        bed_result.lines[0].len()
    );

    for (i, (bb_line, bed_line)) in bb_result
        .lines[0]
        .iter()
        .zip(bed_result.lines[0].iter())
        .enumerate()
    {
        assert_eq!(
            bb_line, bed_line,
            "Line {} differs:\nBigBed: {}\nBED:    {}",
            i, bb_line, bed_line
        );
    }
}

#[tokio::test]
async fn bigbed_search_chr1_small_region() {
    let _guard = IndexGuard::new(BED_INDEX_PATH);
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::bigwig::bigbed_search::bigbed_search;
    use seqa_core::stores::StoreService;
    use seqa_core::tabix::tabix_search::tabix_search;

    let bb_options = SearchOptions::new(BIGBED_PATH, &vec!["chr1:65000-72000".to_string()])
        .unwrap()
        .set_include_header(false);

    let store_service = StoreService::new();
    let bb_result = bigbed_search(&store_service, &bb_options)
        .await
        .expect("Failed to search BigBed");

    let bed_options = SearchOptions::new(BED_PATH, &vec!["chr1:65000-72000".to_string()])
        .unwrap()
        .set_index_path(BED_INDEX_PATH)
        .set_output_format("bed")
        .set_include_header(false);

    let bed_result = tabix_search(&store_service, &bed_options)
        .await
        .expect("Failed to search BED");

    assert_eq!(bb_result.lines[0].len(), bed_result.lines[0].len());

    for (bb_line, bed_line) in bb_result.lines[0].iter().zip(bed_result.lines[0].iter()) {
        assert_eq!(bb_line, bed_line);
    }
}

#[tokio::test]
async fn bigbed_search_different_chromosome() {
    let _guard = IndexGuard::new(BED_INDEX_PATH);
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::bigwig::bigbed_search::bigbed_search;
    use seqa_core::stores::StoreService;
    use seqa_core::tabix::tabix_search::tabix_search;

    let bb_options = SearchOptions::new(BIGBED_PATH, &vec!["chr2:1000000-2000000".to_string()])
        .unwrap()
        .set_include_header(false);

    let store_service = StoreService::new();
    let bb_result = bigbed_search(&store_service, &bb_options)
        .await
        .expect("Failed to search BigBed");

    let bed_options = SearchOptions::new(BED_PATH, &vec!["chr2:1000000-2000000".to_string()])
        .unwrap()
        .set_index_path(BED_INDEX_PATH)
        .set_output_format("bed")
        .set_include_header(false);

    let bed_result = tabix_search(&store_service, &bed_options)
        .await
        .expect("Failed to search BED");

    assert_eq!(bb_result.lines[0].len(), bed_result.lines[0].len());

    for (bb_line, bed_line) in bb_result.lines[0].iter().zip(bed_result.lines[0].iter()) {
        assert_eq!(bb_line, bed_line);
    }
}

#[tokio::test]
async fn bigbed_returns_correct_coordinates() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::bigwig::bigbed_search::bigbed_search;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(BIGBED_PATH, &vec!["chr1:1000000-1100000".to_string()])
        .unwrap()
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = bigbed_search(&store_service, &options)
        .await
        .expect("Failed to search BigBed");

    for line in &result.lines[0] {
        let fields: Vec<&str> = line.split('\t').collect();
        let begin: u32 = fields[1].parse().expect("Failed to parse begin");
        let end: u32 = fields[2].parse().expect("Failed to parse end");

        assert!(
            end > 1000000 && begin < 1100000,
            "Record {}:{}-{} does not overlap query region 1000000-1100000",
            fields[0],
            begin,
            end
        );
    }
}

#[tokio::test]
async fn bigbed_unknown_chromosome_returns_empty_lines() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::bigwig::bigbed_search::bigbed_search;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(
        BIGBED_PATH,
        &vec!["chrM:1-16000".to_string(), "chr1:1000000-1100000".to_string()],
    )
    .unwrap()
    .set_include_header(false);

    let store_service = StoreService::new();
    let result = bigbed_search(&store_service, &options)
        .await
        .expect("Failed to search BigBed");
    assert_eq!(result.coordinates.len(), 2);
    assert_eq!(result.lines.len(), 2);
    assert!(result.lines[0].is_empty());
    assert!(!result.lines[1].is_empty());
}

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

use seqa_core::api::search::search_features;

const S3_FASTA: &str = "s3://com.gmail.docarw/test_data/grch38.fa";
const S3_FASTA_INDEX: &str = "s3://com.gmail.docarw/test_data/grch38.fa.fai";

#[tokio::test]
async fn fasta_chr1() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_FASTA, &vec!["chr1:10001-10100".to_string()])
        .unwrap()
        .set_index_path(S3_FASTA_INDEX)
        .set_output_format("fasta")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .expect("Failed to search FASTA for chr1");
    assert_eq!(result.lines.join("").len(), 100);
}

#[tokio::test]
async fn fasta_chr2() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_FASTA, &vec!["chr2:10001-10100".to_string()])
        .unwrap()
        .set_index_path(S3_FASTA_INDEX)
        .set_output_format("fasta")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .expect("Failed to search FASTA for chr1");
    assert_eq!(result.lines.join("").len(), 100);
}

#[tokio::test]
async fn fasta_multiple_coordinates() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_FASTA, &vec!["chr1:10001-10100".to_string(), "chr2:10001-10100".to_string()])
        .unwrap()
        .set_index_path(S3_FASTA_INDEX)
        .set_output_format("fasta")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .expect("Failed to search FASTA for chr1");
    assert_eq!(result.lines.join("").len(), 200);
}

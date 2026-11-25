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
use std::path::Path;

use seqa_core::api::search::search_features;
use seqa_core::api::search_options::SearchOptions;
use seqa_core::indexes::index_cache::IndexGuard;
use seqa_core::stores::StoreService;
use serial_test::serial;

const S3_VCF: &str = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz";
const S3_VCF_INDEX: &str = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi";
const S3_CACHE_FILE: &str = "s3___com_gmail_docarw_test_data_NA12877_EVA_vcf_gz_tbi";
const GCS_VCF: &str = "gs://genre_test_bucket/NA12877.EVA.vcf.gz";
const GCS_VCF_INDEX: &str = "gs://genre_test_bucket/NA12877.EVA.vcf.gz.tbi";
const GCS_CACHE_FILE: &str = "gs___genre_test_bucket_NA12877_EVA_vcf_gz_tbi";
const LEGACY_CACHE_FILE: &str = "NA12877.EVA.vcf.gz.tbi";
const FIRST_CHR1_LINE: &str = "chr1	116549	.	C	T	.	SuspiciousHomAlt	MTD=bwa_freebayes	GT	1|1";

async fn search_chr1(path: &str, index_path: &str) -> Result<Vec<String>, String> {
    let options = SearchOptions::new(path, &["chr1:1-500000".to_string()])
        .unwrap()
        .set_index_path(index_path)
        .set_output_format("vcf")
        .set_include_header(false);
    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .map_err(|error| error.to_string())?;
    Ok(result.lines[0].clone())
}

fn is_cached(cache_file: &str) -> bool {
    Path::new(cache_file).exists()
}

#[tokio::test]
#[serial(eva_index_cache)]
async fn same_file_name_in_different_locations_is_cached_separately() {
    {
        let _s3_guard = IndexGuard::new(S3_VCF_INDEX);
        let _gcs_guard = IndexGuard::new(GCS_VCF_INDEX);

        let s3_lines = search_chr1(S3_VCF, S3_VCF_INDEX).await.unwrap();
        let gcs_lines = search_chr1(GCS_VCF, GCS_VCF_INDEX).await.unwrap();

        assert_eq!(s3_lines[0], FIRST_CHR1_LINE);
        assert_eq!(gcs_lines, s3_lines);
        assert!(is_cached(S3_CACHE_FILE));
        assert!(is_cached(GCS_CACHE_FILE));
        assert!(!is_cached(LEGACY_CACHE_FILE));
    }
    assert!(!is_cached(S3_CACHE_FILE));
    assert!(!is_cached(GCS_CACHE_FILE));
}

#[tokio::test]
#[serial(eva_index_cache)]
async fn cached_index_is_reused_only_for_its_own_location() {
    let _s3_guard = IndexGuard::new(S3_VCF_INDEX);
    let _gcs_guard = IndexGuard::new(GCS_VCF_INDEX);
    std::fs::write(S3_CACHE_FILE, b"not a tabix index").unwrap();

    let gcs_lines = search_chr1(GCS_VCF, GCS_VCF_INDEX).await.unwrap();
    let s3_result = search_chr1(S3_VCF, S3_VCF_INDEX).await;

    assert_eq!(gcs_lines[0], FIRST_CHR1_LINE);
    assert!(s3_result.is_err(), "the S3 search ignored its cached index");
    assert_eq!(std::fs::read(S3_CACHE_FILE).unwrap(), b"not a tabix index");
}

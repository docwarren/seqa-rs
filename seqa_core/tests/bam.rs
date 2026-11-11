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
use seqa_core::indexes::index_cache::IndexGuard;

const S3_BAM: &str = "s3://com.gmail.docarw/test_data/NA12877.bam";
const S3_BAM_INDEX: &str = "s3://com.gmail.docarw/test_data/NA12877.bam.bai";
#[tokio::test]
async fn bam_chr12() {
    let _guard = IndexGuard::new(S3_BAM_INDEX);
    use seqa_core::api::search::search_features;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;
    let options = SearchOptions::new(S3_BAM, &vec!["chr12:10000000-10000000".to_string()])
        .unwrap()
        .set_index_path(S3_BAM_INDEX)
        .set_output_format("bam")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .unwrap_or_else(|e| panic!("Failed to search BAM: {}", e));
    assert_eq!(result.lines[0].len(), 51);
    assert_eq!(
        result.lines[0][0].to_string(),
        "HSQ1008:141:D0CC8ACXX:4:2203:18142:64281	83	chr12	9999905	60	101M	=	9999602	-404	ATCAGAGACTAGGTTTGCAACCCCTGCTTTATTTTATTTTATTTTATTTACTTATTTATTTATTTTTGCTTTCCATTTGCTTGGGAAATATTTCTCCATCA	;:5>@>ACA@C?=A;;;<=48=EC=>@>DHF;HGHF<IIIIG>HEFDBB9FBHGHCHF@ECHFEFFBHEAE@BE@DBEFECAIIGGGE>BFC;BDDBD<@@	RG:Z:NA12877	XT:A:U	NM:i:0	SM:i:37	AM:i:37	X0:i:1	X1:i:0	XM:i:0	XO:i:0	XG:i:0	MD:Z:101"
    );
    assert_eq!(
        result.lines[0][50].to_string(),
        "HSQ1008:141:D0CC8ACXX:4:1102:6116:159280	99	chr12	9999998	52	101M	=	10000295	398	CTCCATCACTTTATTTTGAGTCTATGTGTGTCTTTGCACATTCAATGGGTCTCCTGAATACAGCACACCAATGGTTCTTGACTCTTTATCCAATTTGCCAG	@CCFFFFFHHHFFGHIIEIGGHHIJJJJIIGGHJJIEGIJIGJEIIJIGGHGIJJJFHJJJJJJGGIIIGHIII=>CHHHGHFFFD>?CCECCCEEDDDC@	RG:Z:NA12877	XT:A:U	NM:i:0	SM:i:37	AM:i:15	X0:i:1	X1:i:0	XM:i:0	XO:i:0	XG:i:0	MD:Z:101"
    );
}

#[tokio::test]
async fn bam_chr12_more() {
    let _guard = IndexGuard::new(S3_BAM_INDEX);
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::bam::bam_search::bam_search;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_BAM, &vec!["chr12:10000000-10010000".to_string()])
        .unwrap()
        .set_index_path(S3_BAM_INDEX)
        .set_include_header(false)
        .set_output_format("bam");

    let store_service = StoreService::new();
    let result = bam_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search BAM for chr12"));
    assert_eq!(result.lines[0].len(), 5639);
}

#[tokio::test]
async fn bam_chr1() {
    let _guard = IndexGuard::new(S3_BAM_INDEX);
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::bam::bam_search::bam_search;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_BAM, &vec!["chr1:10000000-10200000".to_string()])
        .unwrap()
        .set_index_path(S3_BAM_INDEX)
        .set_include_header(false)
        .set_output_format("bam");

    let store_service = StoreService::new();
    let result = bam_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search BAM for chr1"));
    assert_eq!(result.lines[0].len(), 108198);
}

#[tokio::test]
async fn bam_chr_m() {
    let _guard = IndexGuard::new(S3_BAM_INDEX);
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::bam::bam_search::bam_search;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_BAM, &vec!["chrM:1-1".to_string()])
        .unwrap()
        .set_index_path(S3_BAM_INDEX)
        .set_include_header(false)
        .set_output_format("bam");

    let store_service = StoreService::new();
    let result = bam_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search BAM for chrM"));

    assert_eq!(result.lines[0].len(), 120);
    assert_eq!(
        format!("{}", result.lines[0][0]),
        "HSQ1008:141:D0CC8ACXX:3:1308:20201:36071	117	chrM	1	0	*	=	1	0	TGGTTAATAGGGTGATAGACCTGTGATCCATCGTGATGTCTTATTTAAGGGGAACGTGTGGGCTATTTAGGCTTTATGGCCCTGAAGTAGGAACCAGATGT	:C@@C>C<?AA;;;>;;;3?@;>EBBFFDE=HHC@;HHEEIGG:GEBHF@HGGIHF@GDGG?C@DIHEGHHHDG@HGHDIIGEGFAECBHHDDFFBFF@@?	RG:Z:NA12877"
    );
    assert_eq!(
        format!("{}", result.lines[0][119]),
        "HSQ1008:141:D0CC8ACXX:4:2306:19106:26897	1105	chrM	1	37	101M	=	16350	16248	GATCACAGGTCTATCACCCTATTAACCACTCACGGGAGCTCTCCATGCATTTGGTATTTTCGTCTGGGGGGTATGCACGCGATAGCATTGCGAGACGCTGG	AEEC@CCCCCADAA?9BCCDDCA8:8::?<>?CBCCCBCCBCBCECCCEACEEFFFFEA:GHGHGIIGIGIIIEHF:IIIGHHGGG:EDGFHADFFFFCCC	RG:Z:NA12877	XT:A:U	NM:i:1	SM:i:37	AM:i:37	X0:i:1	X1:i:0	XM:i:1	XO:i:0	XG:i:0	MD:Z:72G28"
    );
}

#[tokio::test]
async fn bam_multiple_coordinates() {
    let _guard = IndexGuard::new(S3_BAM_INDEX);
    use seqa_core::api::search::search_features;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;
    let options = SearchOptions::new(
        S3_BAM,
        &vec!["chr1:10000000-10000000".to_string(), "chr12:10000000-10000000".to_string()],
    )
    .unwrap()
    .set_index_path(S3_BAM_INDEX)
    .set_output_format("bam")
    .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .unwrap_or_else(|e| panic!("Failed to search BAM: {}", e));
    assert_eq!(result.lines.len(), 2);
    assert_eq!(result.coordinates.len(), 2);
    assert_eq!(result.lines[0].len(), 48);
    assert_eq!(result.lines[1].len(), 51);
}

#[tokio::test]
async fn bam_invalid_coordinates_return_empty_lines() {
    let _guard = IndexGuard::new(S3_BAM_INDEX);
    use seqa_core::api::search::search_features;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;
    let options = SearchOptions::new(
        S3_BAM,
        &vec![
            "chr1:1-300000".to_string(),
            "chr1:10000000-10000000".to_string(),
            "chr12:1-300000".to_string(),
        ],
    )
    .unwrap()
    .set_index_path(S3_BAM_INDEX)
    .set_output_format("bam")
    .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .unwrap_or_else(|e| panic!("Failed to search BAM: {}", e));
    assert_eq!(result.coordinates.len(), 3);
    assert_eq!(result.lines.len(), 3);
    assert!(result.lines[0].is_empty());
    assert_eq!(result.lines[1].len(), 48);
    assert!(result.lines[2].is_empty());
}

#[tokio::test]
async fn bam_header_only_returns_empty_lines() {
    let _guard = IndexGuard::new(S3_BAM_INDEX);
    use seqa_core::api::search::search_features;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;
    let options = SearchOptions::new(
        S3_BAM,
        &vec!["chr1:10000000-10000000".to_string(), "chr12:10000000-10000000".to_string()],
    )
    .unwrap()
    .set_index_path(S3_BAM_INDEX)
    .set_output_format("bam")
    .set_header_only(true);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .unwrap_or_else(|e| panic!("Failed to search BAM: {}", e));
    assert_eq!(result.coordinates.len(), 2);
    assert_eq!(result.lines.len(), 2);
    assert!(result.lines[0].is_empty());
    assert!(result.lines[1].is_empty());
}

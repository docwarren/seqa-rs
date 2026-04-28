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

const S3_BAM: &str = "s3://com.gmail.docarw/test_data/NA12877.bam";
const S3_BAM_INDEX: &str = "s3://com.gmail.docarw/test_data/NA12877.bam.bai";

fn cleanup_bam_index() {
    seqa_core::indexes::index_cache::delete_local_index(S3_BAM_INDEX);
}

#[tokio::test]
async fn bam_chr12() {
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_BAM, "chr12:10000000-10000000")
        .set_index_path(S3_BAM_INDEX)
        .set_output_format("bam")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = store_service.search_features(&options).await.expect(&format!("Failed to search BAM for chr12: {}", options.chromosome));
    assert_eq!(result.lines.len(), 51);
    assert_eq!(format!("{}", result.lines[0]), "HSQ1008:141:D0CC8ACXX:4:2203:18142:64281	83	chr12	9999905	60	101M	=	9999602	-404	ATCAGAGACTAGGTTTGCAACCCCTGCTTTATTTTATTTTATTTTATTTACTTATTTATTTATTTTTGCTTTCCATTTGCTTGGGAAATATTTCTCCATCA	;:5>@>ACA@C?=A;;;<=48=EC=>@>DHF;HGHF<IIIIG>HEFDBB9FBHGHCHF@ECHFEFFBHEAE@BE@DBEFECAIIGGGE>BFC;BDDBD<@@	RG:Z:NA12877	XT:A:U	NM:i:0	SM:i:37	AM:i:37	X0:i:1	X1:i:0	XM:i:0	XO:i:0	XG:i:0	MD:Z:101");
    assert_eq!(format!("{}", result.lines[50]), "HSQ1008:141:D0CC8ACXX:4:1102:6116:159280	99	chr12	9999998	52	101M	=	10000295	398	CTCCATCACTTTATTTTGAGTCTATGTGTGTCTTTGCACATTCAATGGGTCTCCTGAATACAGCACACCAATGGTTCTTGACTCTTTATCCAATTTGCCAG	@CCFFFFFHHHFFGHIIEIGGHHIJJJJIIGGHJJIEGIJIGJEIIJIGGHGIJJJFHJJJJJJGGIIIGHIII=>CHHHGHFFFD>?CCECCCEEDDDC@	RG:Z:NA12877	XT:A:U	NM:i:0	SM:i:37	AM:i:15	X0:i:1	X1:i:0	XM:i:0	XO:i:0	XG:i:0	MD:Z:101");
    cleanup_bam_index();
}

#[tokio::test]
async fn bam_chr12_more() {
    use seqa_core::api::bam_search::bam_search;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_BAM, "chr12:10000000-10010000")
        .set_index_path(S3_BAM_INDEX)
        .set_include_header(false)
        .set_output_format("bam");

    let store_service = StoreService::new();
    let result = bam_search(&store_service, &options).await.expect(&format!("Failed to search BAM for chr12: {}", options.chromosome));
    assert_eq!(result.lines.len(), 5639);
    cleanup_bam_index();
}

#[tokio::test]
async fn bam_chr1() {
    use seqa_core::api::bam_search::bam_search;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_BAM, "chr1:10000000-10200000")
        .set_index_path(S3_BAM_INDEX)
        .set_include_header(false)
        .set_output_format("bam");


    let store_service = StoreService::new();
    let result = bam_search(&store_service, &options).await.expect(&format!("Failed to search BAM for chr1: {}", options.chromosome));
    assert_eq!(result.lines.len(), 108198);
    cleanup_bam_index();
}

#[tokio::test]
async fn bam_chr_m() {
    use seqa_core::api::bam_search::bam_search;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::stores::StoreService;

    let options = SearchOptions::new(S3_BAM, "chrM:1-1")
        .set_index_path(S3_BAM_INDEX)
        .set_include_header(false)
        .set_output_format("bam");

    let store_service = StoreService::new();
    let result = bam_search(&store_service, &options).await.expect(&format!("Failed to search BAM for chrM: {}", options.chromosome));
    assert_eq!(result.lines.len(), 120);
    assert_eq!(format!("{}", result.lines[0]), "HSQ1008:141:D0CC8ACXX:3:1308:20201:36071	117	chrM	1	0	*	=	1	0	TGGTTAATAGGGTGATAGACCTGTGATCCATCGTGATGTCTTATTTAAGGGGAACGTGTGGGCTATTTAGGCTTTATGGCCCTGAAGTAGGAACCAGATGT	:C@@C>C<?AA;;;>;;;3?@;>EBBFFDE=HHC@;HHEEIGG:GEBHF@HGGIHF@GDGG?C@DIHEGHHHDG@HGHDIIGEGFAECBHHDDFFBFF@@?	RG:Z:NA12877");
    assert_eq!(format!("{}", result.lines[119]), "HSQ1008:141:D0CC8ACXX:4:2306:19106:26897	1105	chrM	1	37	101M	=	16350	16248	GATCACAGGTCTATCACCCTATTAACCACTCACGGGAGCTCTCCATGCATTTGGTATTTTCGTCTGGGGGGTATGCACGCGATAGCATTGCGAGACGCTGG	AEEC@CCCCCADAA?9BCCDDCA8:8::?<>?CBCCCBCCBCBCECCCEACEEFFFFEA:GHGHGIIGIGIIIEHF:IIIGHHGGG:EDGFHADFFFFCCC	RG:Z:NA12877	XT:A:U	NM:i:1	SM:i:37	AM:i:37	X0:i:1	X1:i:0	XM:i:1	XO:i:0	XG:i:0	MD:Z:72G28");
    cleanup_bam_index();
}

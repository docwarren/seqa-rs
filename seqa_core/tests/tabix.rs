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
use seqa_core::api::search_options::SearchOptions;
use seqa_core::indexes::index_cache::{delete_local_index, get_local_index_path};
use seqa_core::stores::StoreService;
use seqa_core::tabix::tabix_search::tabix_search;

const S3_VCF: &str = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz";
const S3_VCF_INDEX: &str = "s3://com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi";
const S3_CNV_VCF: &str = "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz";
const S3_CNV_VCF_INDEX: &str = "s3://com.gmail.docarw/test_data/NA12878.gatk.cnv.vcf.gz.tbi";
const S3_GTF: &str = "s3://com.gmail.docarw/test_data/genes_exon_sorted.gtf.gz";
const S3_GTF_INDEX: &str = "s3://com.gmail.docarw/test_data/genes_exon_sorted.gtf.gz.tbi";
const AZ_VCF: &str = "az://genreblobs/genre-test-data/NA12877.EVA.vcf.gz";
const AZ_VCF_INDEX: &str = "az://genreblobs/genre-test-data/NA12877.EVA.vcf.gz.tbi";
const GCS_VCF: &str = "gs://genre_test_bucket/NA12877.EVA.vcf.gz";
const GCS_VCF_INDEX: &str = "gs://genre_test_bucket/NA12877.EVA.vcf.gz.tbi";

// In common/mod.rs
pub struct IndexGuard(&'static str);

impl IndexGuard {
    pub fn new(path: &'static str) -> Self {
        Self(path)
    }
}

impl Drop for IndexGuard {
    fn drop(&mut self) {
        delete_local_index(self.0);
    }
}

#[tokio::test]
async fn vcf_chr1() {
    let _guard = IndexGuard::new(S3_VCF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_VCF, &vec!["chr1:1-500000".to_string()])
        .unwrap()
        .set_index_path(S3_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr1"));
    assert_eq!(result.lines.len(), 14);
    assert_eq!(
        result.lines[0],
        "chr1	116549	.	C	T	.	SuspiciousHomAlt	MTD=bwa_freebayes	GT	1|1"
    );
    assert_eq!(
        result.lines[13],
        "chr1	356537	.	G	A	.	SuspiciousHomAlt	MTD=cgi	GT	1|1"
    );
}

#[tokio::test]
async fn vcf_chr12() {
    let _guard = IndexGuard::new(S3_VCF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_VCF, &vec!["chr12:1-120000".to_string()])
        .unwrap()
        .set_index_path(S3_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr12"));
    assert_eq!(result.lines.len(), 3);
    assert_eq!(
        result.lines[0],
        "chr12	86886	.	G	C	.	PASS	MTD=isaac2,bwa_freebayes,bwa_platypus,bwa_gatk3	GT	1|1"
    );
    assert_eq!(
        result.lines[2],
        "chr12	91430	.	T	A	.	SuspiciousHomAlt	MTD=bwa_freebayes,bwa_platypus,bwa_gatk3	GT	1|1"
    );
}

#[tokio::test]
async fn vcf_chr_x() {
    let _guard = IndexGuard::new(S3_VCF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_VCF, &vec!["chrX:154927181-154929412".to_string()])
        .unwrap()
        .set_index_path(S3_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chrX"));
    assert_eq!(result.lines.len(), 6);
    assert_eq!(
        result.lines[0],
        "chrX	154927181	.	C	CCAC,T	.	PASS	MTD=bwa_platypus	GT	1"
    );
    assert_eq!(
        result.lines[5],
        "chrX	154929412	.	C	T	.	PASS	MTD=bwa_freebayes,bwa_platypus,cortex,bwa_gatk3	GT	1"
    );
}

#[tokio::test]
async fn vcf_chr4() {
    let _guard = IndexGuard::new(S3_VCF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_VCF, &vec!["chr4:4928400-4928402".to_string()])
        .unwrap()
        .set_index_path(S3_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr4"));
    assert_eq!(result.lines.len(), 1);
    assert_eq!(
        result.lines[0],
        "chr4	4928401	.	C	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	0|1"
    );
}

#[tokio::test]
async fn vcf_multiple_coords() {
    let _guard = IndexGuard::new(S3_VCF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_VCF, &vec!["chr12:1-120000".to_string(), "chrX:154927181-154929412".to_string()])
        .unwrap()
        .set_index_path(S3_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr4"));
    assert_eq!(result.lines.len(), 9);
    assert_eq!(
        result.lines[0],
        "chr12	86886	.	G	C	.	PASS	MTD=isaac2,bwa_freebayes,bwa_platypus,bwa_gatk3	GT	1|1"
    );
    assert_eq!(
        result.lines[8],
        "chrX	154929412	.	C	T	.	PASS	MTD=bwa_freebayes,bwa_platypus,cortex,bwa_gatk3	GT	1"
    );
}

#[tokio::test]
async fn vcf_many_lines() {
    let _guard = IndexGuard::new(S3_VCF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_VCF, &vec!["chr1:100000000-200000000".to_string()])
        .unwrap()
        .set_index_path(S3_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr1"));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(
        result.lines[0],
        "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1"
    );
    assert_eq!(
        result.lines[112929],
        "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1"
    );
}

#[tokio::test]
async fn cnv_vcf_chr_12() {
    let _guard = IndexGuard::new(S3_CNV_VCF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_CNV_VCF, &vec!["chr12:1-100000000".to_string()])
        .unwrap()
        .set_index_path(S3_CNV_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr12"));
    assert_eq!(result.lines.len(), 250);
    assert_eq!(
        result.lines[0],
        "chr12	16000	NA12878_DUP_chr12_1	G	<DUP>	999	DEPTH_LT_5KB;REF_PANEL_GENOTYPES	ALGORITHMS=depth;CHR2=chr12;END=18000;EVIDENCE=RD;SVLEN=2000;SVTYPE=DUP;PROTEIN_CODING__NEAREST_TSS=IQSEC3;PROTEIN_CODING__INTERGENIC;AN=314;AC=226;AF=0.719745;N_BI_GENOS=157;N_HOMREF=24;N_HET=40;N_HOMALT=93;FREQ_HOMREF=0.152866;FREQ_HET=0.254777;FREQ_HOMALT=0.592357	GT:EV:GQ:RD_CN:RD_GQ	0/1:RD:21:3:21"
    );
    assert_eq!(
        result.lines[249],
        "chr12	99894566	NA12878_DEL_chr12_124	A	<DEL>	951	PASS	ALGORITHMS=depth,manta,wham;BOTHSIDES_SUPPORT;CHR2=chr12;END=99898009;EVIDENCE=PE,RD,SR;SVLEN=3443;SVTYPE=DEL;PROTEIN_CODING__INTRONIC=ANKS1B;NONCODING_SPAN=DNase;NONCODING_BREAKPOINT=Tommerup_TADanno;AN=314;AC=8;AF=0.025478;N_BI_GENOS=157;N_HOMREF=149;N_HET=8;N_HOMALT=0;FREQ_HOMREF=0.949045;FREQ_HET=0.0509554;FREQ_HOMALT=0;gnomad_v2.1_sv_SVID=.;gnomad_v2.1_sv_AF=0.025629;gnomad_v2.1_sv_AFR_AF=0.014055;gnomad_v2.1_sv_AMR_AF=0.029534;gnomad_v2.1_sv_EAS_AF=0;gnomad_v2.1_sv_EUR_AF=0.047613	GT:EV:GQ:PE_GQ:PE_GT:RD_CN:RD_GQ:SR_GQ:SR_GT	0/1:RD,PE,SR:99:260:1:1:125:791:1"
    );
}

#[tokio::test]
async fn gff_test() {
    let _guard = IndexGuard::new(S3_GTF_INDEX); // Automatically deletes index on scope exit, even on panic!
    let options = SearchOptions::new(S3_GTF, &vec!["chr1:1-50000".to_string()])
        .unwrap()
        .set_index_path(S3_GTF_INDEX)
        .set_output_format("gff")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .expect(&format!("Failed to search GFF for chr1"));
    assert_eq!(result.lines.len(), 31);
}

#[tokio::test]
async fn gtf_test() {
    let _guard = IndexGuard::new(S3_GTF_INDEX);
    let options = SearchOptions::new(S3_GTF, &vec!["chr1:1-50000".to_string()])
        .unwrap()
        .set_index_path(S3_GTF_INDEX)
        .set_output_format("gtf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .expect(&format!("Failed to search GTF for chr1",));
    assert_eq!(result.lines.len(), 31);
}

#[tokio::test]
async fn azure_vcf() {
    let _guard = IndexGuard::new(AZ_VCF_INDEX);
    let options = SearchOptions::new(AZ_VCF, &vec!["chr1:100000000-200000000".to_string()])
        .unwrap()
        .set_index_path(AZ_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr1"));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(
        result.lines[0],
        "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1"
    );
    assert_eq!(
        result.lines[112929],
        "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1"
    );
}

#[tokio::test]
async fn gc_vcf() {
    let _guard = IndexGuard::new(GCS_VCF_INDEX);
    let options = SearchOptions::new(GCS_VCF, &vec!["chr1:100000000-200000000".to_string()])
        .unwrap()
        .set_index_path(GCS_VCF_INDEX)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr1"));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(
        result.lines[0],
        "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1"
    );
    assert_eq!(
        result.lines[112929],
        "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1"
    );
}

#[tokio::test]
#[serial_test::serial(bedgraph_cache)]
async fn no_cache_does_not_write_index_file() {
    let no_cache_file = "s3://com.gmail.docarw/test_data/test.bedgraph.gz";
    let no_cache_index = "s3://com.gmail.docarw/test_data/test.bedgraph.gz.tbi";
    let _guard = IndexGuard::new(no_cache_index);

    let cache_path = get_local_index_path(no_cache_index);
    assert!(
        !cache_path.exists(),
        "Cache file should not exist before search"
    );

    let options = SearchOptions::new(no_cache_file, &vec!["chr1:1-100000000".to_string()])
        .unwrap()
        .set_index_path(no_cache_index)
        .set_output_format("bedgraph")
        .set_include_header(false)
        .set_no_cache(true);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect("Failed to search BEDGRAPH");
    assert!(!result.lines.is_empty(), "Search should return results");
    assert!(
        !cache_path.exists(),
        "Cache file should not exist after search with no_cache=true"
    );
}

#[tokio::test]
async fn http_vcf() {
    let http_vcf =
        "https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test_data/NA12877.EVA.vcf.gz";
    let http_vcf_index =
        "https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi";
    let _guard = IndexGuard::new(http_vcf_index);
    let options = SearchOptions::new(http_vcf, &vec!["chr1:100000000-200000000".to_string()])
        .unwrap()
        .set_index_path(http_vcf_index)
        .set_output_format("vcf")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = tabix_search(&store_service, &options)
        .await
        .expect(&format!("Failed to search VCF for chr1"));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(
        result.lines[0],
        "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1"
    );
    assert_eq!(
        result.lines[112929],
        "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1"
    );
}

#[tokio::test]
#[serial_test::serial(bedgraph_cache)]
async fn bedgraph_test() {
    let bedgraph = "s3://com.gmail.docarw/test_data/test.bedgraph.gz";
    let bedgraph_index = "s3://com.gmail.docarw/test_data/test.bedgraph.gz.tbi";
    let _guard = IndexGuard::new(bedgraph_index);
    let options = SearchOptions::new(bedgraph, &vec!["chr1:1-100000000".to_string()])
        .unwrap()
        .set_index_path(bedgraph_index)
        .set_output_format("bedgraph")
        .set_include_header(false);

    let store_service = StoreService::new();
    let result = search_features(&store_service, &options)
        .await
        .expect("Failed to search BEDGRAPH for chr1");
    assert_eq!(result.lines.len(), 5860);
}

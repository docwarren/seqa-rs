use seqa_core::indexes::index_cache::delete_local_index;

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

#[tokio::test]
async fn vcf_chr1() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(S3_VCF)
        .set_index_path(S3_VCF_INDEX)
        .set_chromosome("chr1")
        .set_begin(1)
        .set_end(500000)
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr1: {}", options.chromosome));
    assert_eq!(result.lines.len(), 14);
    assert_eq!(result.lines[0], "chr1	116549	.	C	T	.	SuspiciousHomAlt	MTD=bwa_freebayes	GT	1|1");
    assert_eq!(result.lines[13], "chr1	356537	.	G	A	.	SuspiciousHomAlt	MTD=cgi	GT	1|1");
    delete_local_index(S3_VCF_INDEX);
}

#[tokio::test]
async fn vcf_chr12() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(S3_VCF)
        .set_index_path(S3_VCF_INDEX)
        .set_chromosome("chr12")
        .set_begin(1)
        .set_end(120000)
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr12: {}", options.chromosome));
    assert_eq!(result.lines.len(), 3);
    assert_eq!(result.lines[0], "chr12	86886	.	G	C	.	PASS	MTD=isaac2,bwa_freebayes,bwa_platypus,bwa_gatk3	GT	1|1");
    assert_eq!(result.lines[2], "chr12	91430	.	T	A	.	SuspiciousHomAlt	MTD=bwa_freebayes,bwa_platypus,bwa_gatk3	GT	1|1");
    delete_local_index(S3_VCF_INDEX);
}

#[tokio::test]
async fn vcf_chrx() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(S3_VCF)
        .set_index_path(S3_VCF_INDEX)
        .set_coordinates("chrX:154927181-154929412")
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chrX: {}", options.chromosome));
    assert_eq!(result.lines.len(), 6);
    assert_eq!(result.lines[0], "chrX	154927181	.	C	CCAC,T	.	PASS	MTD=bwa_platypus	GT	1");
    assert_eq!(result.lines[5], "chrX	154929412	.	C	T	.	PASS	MTD=bwa_freebayes,bwa_platypus,cortex,bwa_gatk3	GT	1");
    delete_local_index(S3_VCF_INDEX);
}

#[tokio::test]
async fn vcf_chr4() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(S3_VCF)
        .set_index_path(S3_VCF_INDEX)
        .set_coordinates("chr4:4928400-4928402")
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr4: {}", options.chromosome));
    assert_eq!(result.lines.len(), 1);
    assert_eq!(result.lines[0], "chr4	4928401	.	C	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	0|1");
    delete_local_index(S3_VCF_INDEX);
}

#[tokio::test]
async fn vcf_many_lines() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(S3_VCF)
        .set_index_path(S3_VCF_INDEX)
        .set_coordinates("chr1:100000000-200000000")
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr1: {}", options.chromosome));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(result.lines[0], "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1");
    assert_eq!(result.lines[112929], "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1");
    delete_local_index(S3_VCF_INDEX);
}

#[tokio::test]
async fn cnv_vcf_chr_12() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(S3_CNV_VCF)
        .set_index_path(S3_CNV_VCF_INDEX)
        .set_chromosome("chr12")
        .set_begin(1)
        .set_end(100000000)
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr12: {}", options.chromosome));
    assert_eq!(result.lines.len(), 250);
    assert_eq!(result.lines[0], "chr12	16000	NA12878_DUP_chr12_1	G	<DUP>	999	DEPTH_LT_5KB;REF_PANEL_GENOTYPES	ALGORITHMS=depth;CHR2=chr12;END=18000;EVIDENCE=RD;SVLEN=2000;SVTYPE=DUP;PROTEIN_CODING__NEAREST_TSS=IQSEC3;PROTEIN_CODING__INTERGENIC;AN=314;AC=226;AF=0.719745;N_BI_GENOS=157;N_HOMREF=24;N_HET=40;N_HOMALT=93;FREQ_HOMREF=0.152866;FREQ_HET=0.254777;FREQ_HOMALT=0.592357	GT:EV:GQ:RD_CN:RD_GQ	0/1:RD:21:3:21");
    assert_eq!(result.lines[249], "chr12	99894566	NA12878_DEL_chr12_124	A	<DEL>	951	PASS	ALGORITHMS=depth,manta,wham;BOTHSIDES_SUPPORT;CHR2=chr12;END=99898009;EVIDENCE=PE,RD,SR;SVLEN=3443;SVTYPE=DEL;PROTEIN_CODING__INTRONIC=ANKS1B;NONCODING_SPAN=DNase;NONCODING_BREAKPOINT=Tommerup_TADanno;AN=314;AC=8;AF=0.025478;N_BI_GENOS=157;N_HOMREF=149;N_HET=8;N_HOMALT=0;FREQ_HOMREF=0.949045;FREQ_HET=0.0509554;FREQ_HOMALT=0;gnomad_v2.1_sv_SVID=.;gnomad_v2.1_sv_AF=0.025629;gnomad_v2.1_sv_AFR_AF=0.014055;gnomad_v2.1_sv_AMR_AF=0.029534;gnomad_v2.1_sv_EAS_AF=0;gnomad_v2.1_sv_EUR_AF=0.047613	GT:EV:GQ:PE_GQ:PE_GT:RD_CN:RD_GQ:SR_GQ:SR_GT	0/1:RD,PE,SR:99:260:1:1:125:791:1");
    delete_local_index(S3_CNV_VCF_INDEX);
}

#[tokio::test]
async fn gff_test() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(S3_GTF)
        .set_index_path(S3_GTF_INDEX)
        .set_chromosome("chr1")
        .set_begin(1)
        .set_end(50000)
        .set_output_format("gff")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search GFF for chr1: {}", options.chromosome));
    assert_eq!(result.lines.len(), 31);
    delete_local_index(S3_GTF_INDEX);
}

#[tokio::test]
async fn azure_vcf() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(AZ_VCF)
        .set_index_path(AZ_VCF_INDEX)
        .set_coordinates("chr1:100000000-200000000")
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr1: {}", options.chromosome));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(result.lines[0], "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1");
    assert_eq!(result.lines[112929], "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1");
    delete_local_index(AZ_VCF_INDEX);
}

#[tokio::test]
async fn gc_vcf() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(GCS_VCF)
        .set_index_path(GCS_VCF_INDEX)
        .set_coordinates("chr1:100000000-200000000")
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr1: {}", options.chromosome));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(result.lines[0], "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1");
    assert_eq!(result.lines[112929], "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1");
    delete_local_index(GCS_VCF_INDEX);
}

#[tokio::test]
async fn no_cache_does_not_write_index_file() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;
    use seqa_core::indexes::index_cache::get_local_index_path;

    // Clean up any leftover cache file from previous runs
    delete_local_index(S3_VCF_INDEX);

    let cache_path = get_local_index_path(S3_VCF_INDEX);
    assert!(!cache_path.exists(), "Cache file should not exist before search");

    let options = SearchOptions::new()
        .set_file_path(S3_VCF)
        .set_index_path(S3_VCF_INDEX)
        .set_coordinates("chr1:116549-116550")
        .set_output_format("vcf")
        .set_include_header(false)
        .set_no_cache(true);

    let result = tabix_search(&options).await.expect("Failed to search VCF");
    assert!(!result.lines.is_empty(), "Search should return results");
    assert!(!cache_path.exists(), "Cache file should not exist after search with no_cache=true");
}

#[tokio::test]
async fn http_vcf() {
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let http_vcf = "https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test_data/NA12877.EVA.vcf.gz";
    let http_vcf_index = "https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test_data/NA12877.EVA.vcf.gz.tbi";

    let options = SearchOptions::new()
        .set_file_path(http_vcf)
        .set_index_path(http_vcf_index)
        .set_coordinates("chr1:100000000-200000000")
        .set_output_format("vcf")
        .set_include_header(false);

    let result = tabix_search(&options).await.expect(&format!("Failed to search VCF for chr1: {}", options.chromosome));
    assert_eq!(result.lines.len(), 112930);
    assert_eq!(result.lines[0], "chr1	100006117	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,isaac2,bwa_gatk3	GT	1|1");
    assert_eq!(result.lines[112929], "chr1	199999917	.	G	A	.	PASS	MTD=cgi,bwa_freebayes,bwa_platypus,bwa_gatk3,cortex,isaac2	GT	0|1");
    delete_local_index(http_vcf_index);
}

/// Tests for BigBed file search functionality.
/// Compares bigbed_search results against tabix_search on equivalent BED data.

use seqa_core::indexes::index_cache::delete_local_index;

const BIGBED_PATH: &str = "s3://com.soma23.data/hg38/mane.bb";
const BED_PATH: &str = "s3://com.soma23.data/hg38/mane.bed.gz";
const BED_INDEX_PATH: &str = "s3://com.soma23.data/hg38/mane.bed.gz.tbi";

fn cleanup_bed_index() {
    delete_local_index(BED_INDEX_PATH);
}

#[tokio::test]
async fn bigbed_search_matches_tabix() {
    use seqa_core::api::bigbed_search::bigbed_search;
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let bb_options = SearchOptions::new()
        .set_file_path(BIGBED_PATH)
        .set_coordinates("chr1:1000000-1300000")
        .set_include_header(false);

    let bb_result = bigbed_search(&bb_options).await.expect("Failed to search BigBed");

    let bed_options = SearchOptions::new()
        .set_file_path(BED_PATH)
        .set_index_path(BED_INDEX_PATH)
        .set_coordinates("chr1:1000000-1300000")
        .set_output_format("bed")
        .set_include_header(false);

    let bed_result = tabix_search(&bed_options).await.expect("Failed to search BED");

    assert_eq!(
        bb_result.lines.len(),
        bed_result.lines.len(),
        "BigBed returned {} lines, BED returned {} lines",
        bb_result.lines.len(),
        bed_result.lines.len()
    );

    for (i, (bb_line, bed_line)) in bb_result.lines.iter().zip(bed_result.lines.iter()).enumerate() {
        assert_eq!(
            bb_line, bed_line,
            "Line {} differs:\nBigBed: {}\nBED:    {}",
            i, bb_line, bed_line
        );
    }
    cleanup_bed_index();
}

#[tokio::test]
async fn bigbed_search_chr1_small_region() {
    use seqa_core::api::bigbed_search::bigbed_search;
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let bb_options = SearchOptions::new()
        .set_file_path(BIGBED_PATH)
        .set_coordinates("chr1:65000-72000")
        .set_include_header(false);

    let bb_result = bigbed_search(&bb_options).await.expect("Failed to search BigBed");

    let bed_options = SearchOptions::new()
        .set_file_path(BED_PATH)
        .set_index_path(BED_INDEX_PATH)
        .set_coordinates("chr1:65000-72000")
        .set_output_format("bed")
        .set_include_header(false);

    let bed_result = tabix_search(&bed_options).await.expect("Failed to search BED");

    assert_eq!(bb_result.lines.len(), bed_result.lines.len());

    for (bb_line, bed_line) in bb_result.lines.iter().zip(bed_result.lines.iter()) {
        assert_eq!(bb_line, bed_line);
    }
    cleanup_bed_index();
}

#[tokio::test]
async fn bigbed_search_different_chromosome() {
    use seqa_core::api::bigbed_search::bigbed_search;
    use seqa_core::api::tabix_search::tabix_search;
    use seqa_core::api::search_options::SearchOptions;

    let bb_options = SearchOptions::new()
        .set_file_path(BIGBED_PATH)
        .set_coordinates("chr2:1000000-2000000")
        .set_include_header(false);

    let bb_result = bigbed_search(&bb_options).await.expect("Failed to search BigBed");

    let bed_options = SearchOptions::new()
        .set_file_path(BED_PATH)
        .set_index_path(BED_INDEX_PATH)
        .set_coordinates("chr2:1000000-2000000")
        .set_output_format("bed")
        .set_include_header(false);

    let bed_result = tabix_search(&bed_options).await.expect("Failed to search BED");

    assert_eq!(bb_result.lines.len(), bed_result.lines.len());

    for (bb_line, bed_line) in bb_result.lines.iter().zip(bed_result.lines.iter()) {
        assert_eq!(bb_line, bed_line);
    }
    cleanup_bed_index();
}

#[tokio::test]
async fn bigbed_returns_correct_coordinates() {
    use seqa_core::api::bigbed_search::bigbed_search;
    use seqa_core::api::search_options::SearchOptions;

    let options = SearchOptions::new()
        .set_file_path(BIGBED_PATH)
        .set_coordinates("chr1:1000000-1100000")
        .set_include_header(false);

    let result = bigbed_search(&options).await.expect("Failed to search BigBed");

    for line in &result.lines {
        let fields: Vec<&str> = line.split('\t').collect();
        let begin: u32 = fields[1].parse().expect("Failed to parse begin");
        let end: u32 = fields[2].parse().expect("Failed to parse end");

        assert!(end > 1000000 && begin < 1100000,
            "Record {}:{}-{} does not overlap query region 1000000-1100000",
            fields[0], begin, end
        );
    }
}

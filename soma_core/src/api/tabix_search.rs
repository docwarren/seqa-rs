use thiserror::Error;

use super::search::{init_fetch_handles, join_fetch_handles};
use super::search_options::SearchOptions;
use crate::api::output_format::OutputFormat;
use crate::api::search::SearchError;
use crate::api::search_result::SearchResult;
use crate::indexes::bin_util::get_bin_numbers;
use crate::indexes::tabix::TabixError;
use crate::indexes::traits::sam_index::SamIndex;
use crate::models::bed::BedLine;
use crate::models::bedgraph::BedGraphLine;
use crate::models::gff::GffLine;
use crate::models::gtf::GtfLine;
use crate::models::tabix_header::{TabixHeader, TabixHeaderError};
use crate::{indexes::tabix::Tabix, models::vcf::VcfLine};

#[derive(Debug, Error)]
pub enum TabixSearchError {
    #[error("Search error: {0}")]
    SearchError(String),

    #[error("Tabix index error: {0}")]
    TabixIndexError(#[from] TabixError),

    #[error("async threads error: {0}")]
    AsyncThreadsError(#[from] SearchError),

    #[error("Failed to read tabix header: {0}")]
    FailedToReadTabixHeader(#[from] TabixHeaderError),
}

/// Converts raw data bytes into a vector of strings, processing each line according to the search options.
/// # Arguments:
/// * `data` - A vector of bytes representing the raw data to be processed.
/// * `options` - A `SearchOptions` struct containing the search parameters such as output format,
///  whether to include headers, and the range of positions to consider.
/// # Returns:
/// * A Result containing a vector of strings with the processed lines, which may include headers.
pub fn data_to_lines(data: &Vec<u8>, options: &SearchOptions) -> Vec<String> {
    let raw_string = String::from_utf8_lossy(data).into_owned();
    let line_strings = raw_string
        .split('\n')
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect::<Vec<String>>();

    let model_new = options.output_format.get_model();

    line_strings
        .iter()
        .map(|line| model_new(&line))
        .filter_map(|line| match line {
            Ok(feature) => Some(feature),
            Err(e) => {
                eprintln!("{}", e);
                None
            },
        })
        .filter(|feature| {
            feature.overlaps(options)
        })
        .map(|feature| format!("{}", feature))
        .collect()
}

/// Searches for data in a tabixed bgzipped file based on the provided search options.
/// Returns a vector of strings containing the results, which may include  headers.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the search parameters such as file paths, chromosome,
///  start and end positions, output format, and whether to include headers or only headers.
/// # Returns:
/// * a Result containing a vector of strings with the search results, or an error message if the search fails.
pub async fn tabix_search(options: &SearchOptions) -> Result<SearchResult, TabixSearchError> {
    let mut result = SearchResult::new();

    let tabix = match &options.tabix_index {
        Some(index) => index,
        None => &Tabix::from_compressed_file_with_data_path(&options.index_path, &options.file_path).await?
    };
    let tabix_header = match &options.tabix_header {
        Some(header) => header,
        None => {
            let first_feature_offset = tabix.first_feature_offset.clone();
            &TabixHeader::from_file(&options.file_path, first_feature_offset).await?
        }
    };

    result.tabix_header = Some(tabix_header.clone());
    result.tabix_index = Some(tabix.clone());

    if options.header_only {
        result.lines = tabix_header.to_lines();
        return Ok(result);
    }

    let bin_numbers = get_bin_numbers(options.begin, options.end);
    let chr_i = tabix.get_chromosome_index_by_name(&options.chromosome);
    let chr_i = match chr_i {
        Some(i) => i as u32,
        None => {
            return Err(TabixSearchError::SearchError(format!(
                "Chromosome {} not found in index",
                options.chromosome
            )));
        }
    };
    let chr_idx = &tabix.references[chr_i as usize];
    let chunks = tabix.get_optimized_chunks(&chr_idx, bin_numbers, &options);
    let chunk_handles = init_fetch_handles(&options, &chunks).await?;
    let raw_data = join_fetch_handles(chunk_handles).await?;
    let lines = data_to_lines(&raw_data.concat(), &options);

    let mut all_lines = get_header_lines(options, &tabix_header);
    all_lines.extend(lines);
    result.lines = all_lines;
    return Ok(result);

}

/// Searches for data in a vcf file using the provided search options.
/// Returns a vector of `VcfLine` objects containing the results.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the search parameters such as file
///     paths, chromosome, start and end positions, output format, and whether to include headers or only headers.
/// # Returns:
/// * A  Result containing a vector of `VcfLine` objects with the search results, or an error message if the search fails.
pub async fn tabix_search_vcf(options: &SearchOptions) -> Result<Vec<VcfLine>, TabixSearchError> {
    let tabix_result = tabix_search(options).await?;
    let mut vcf_lines = Vec::new();

    for line in tabix_result.lines {
        if let Ok(vcf_line) = VcfLine::from_line(line) {
            vcf_lines.push(vcf_line);
        }
    }
    Ok(vcf_lines)
}

/// Gets the header lines for the specified output format.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the search parameters such as file
///     paths, chromosome, start and end positions, output format, and whether to include headers or only headers.
/// * `tabix_header` - A `TabixHeader` struct containing the header information for the tabix file.
/// # Returns:
/// * A vector of strings containing the header lines for the specified output format.
pub fn get_header_lines(options: &SearchOptions, tabix_header: &TabixHeader) -> Vec<String> {
    if options.include_header {
        match options.output_format {
            OutputFormat::VCF => tabix_header.to_lines(),
            OutputFormat::GFF => vec![GffLine::COLUMNS.join("\t")],
            OutputFormat::GTF => vec![GtfLine::COLUMNS.join("\t")],
            OutputFormat::BED => vec![BedLine::COLUMNS.join("\t")],
            OutputFormat::BEDGRAPH => vec![BedGraphLine::COLUMNS.join("\t")],
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

#[test]
fn test_get_header_lines() {
    let mut options = SearchOptions::new();
    options.include_header = true;
    options.output_format = OutputFormat::GFF;
    options.file_path = "test.gff.gz".to_string();
    options.index_path = "test.gff.gz.tbi".to_string();
    options.begin = 0;
    options.end = 1000;
    options.chromosome = "chr1".to_string();
    options.header_only = false;
    options.bigwig_index = None;

    let tabix_header = TabixHeader::new();
    let header_lines = get_header_lines(&options, &tabix_header);
    println!("{:?}", header_lines);
    assert!(header_lines.len() == 1);
    assert!(header_lines[0] == "chromosome\tsource\tfeature\tbegin\tend\tscore\tstrand\tframe\tgroup");
}

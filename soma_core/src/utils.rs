use crate::api::output_format::OutputFormat;
use crate::api::search_options::SearchOptions;
use crate::genome::get_longest_possible_genome;
use crate::models::FileSearchRequest;
use std::num::ParseIntError;
use std::path::absolute;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtilError {
    #[error("{0}")]
    FormatError(#[from] FormatError),

    #[error("{0}")]
    ExtensionError(#[from] ExtensionError),

    #[error("Error converting to absolute path: {0}")]
    AbsolutePathError(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("Invalid options format: {0}")]
    InvalidOptions(String),

    #[error("Invalid Coordinate string format")]
    InvalidCoordinateFormat(String),

    #[error("Error parsing coordinates: {0}")]
    ParseIntError(#[from] ParseIntError),
}

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("Error determining index path: {0}")]
    IndexPathError(String),

    #[error("Error determining file type: {0}")]
    PathTypeError(String),
}

pub fn format_file_path(file_path: &str) -> Result<String, UtilError> {
    if file_path.starts_with("/") || file_path.starts_with("./") || file_path.starts_with("../") {
        match std::fs::exists(file_path) {
            Ok(true) => (),
            Ok(false) => {
                return Err(UtilError::FormatError(FormatError::InvalidOptions(
                    format!("File path does not exist: {}", file_path),
                )));
            }
            Err(e) => return Err(UtilError::AbsolutePathError(e)),
        }
        let abs_path = absolute(file_path)?;
        Ok(format!("file://{}", abs_path.to_string_lossy()))
    } else {
        Ok(file_path.to_string())
    }
}

pub fn get_search_options(request: FileSearchRequest) -> Result<SearchOptions, UtilError> {
    // Validate and process the search request
    if request.path.is_empty() || request.coordinates.is_empty() {
        return Err(UtilError::FormatError(FormatError::InvalidOptions(
            format!("File path and coordinates must be provided: file: {}, coords: {}", request.path, request.coordinates),
        )));
    }
    let mut options = SearchOptions::new();

    let (chromosome, begin, end) = parse_coordinates(&request.coordinates)?;
    options.chromosome = chromosome;
    options.begin = begin;
    options.end = end;
    options.file_path = format_file_path(&request.path)?;
    options.index_path = get_index_path(&options.file_path)?;
    options.output_format = get_output_format(&options.file_path)?;
    options.bigwig_index =  request.bigwig_index;
    options.bam_header = request.bam_header;
    options.bam_index = request.bam_index;
    options.tabix_header = request.tabix_header;
    options.tabix_index = request.tabix_index;
    options.fasta_index = request.fasta_index;
    Ok(options)
}

pub fn parse_coordinates(coords: &str) -> Result<(String, u32, u32), FormatError> {
    let longest_genome = get_longest_possible_genome();
    let tokens: Vec<&str> = coords.split(':').collect();

    let chromosome = tokens[0].to_string();
    let chr_idx = crate::genome::chr_index(&chromosome).ok_or_else(|| {
        FormatError::InvalidCoordinateFormat(format!("Invalid chromosome: {}.", chromosome))
    })?;

    let (begin, end) = if tokens.len() == 2 {
        let parts: Vec<String> = tokens[1].split('-').map(|s| s.replace(",", "")).collect();

        if parts.len() == 2 {
            let begin = parts[0].parse::<u32>()?;
            let end = parts[1].parse::<u32>()?;
            (begin, end)
        } else {
            let begin = parts[0].parse::<u32>()?;
            let end = longest_genome[chr_idx];
            (begin, end)
        }
    } else {
        (1, longest_genome[chr_idx])
    };

    Ok((chromosome, begin, end))
}

pub fn get_index_path(file_path: &str) -> Result<String, ExtensionError> {
    let lower_path = file_path.to_ascii_lowercase();
    // Logic to determine the index path based on the file type
    if lower_path.ends_with(".bam") {
        Ok(format!("{}.bai", file_path))
    } else if lower_path.ends_with(".fa") || lower_path.ends_with(".fasta") {
        Ok(format!("{}.fai", file_path))
    } else if lower_path.ends_with(".bigwig") || lower_path.ends_with(".bw") {
        Ok(format!("-"))
    } else if lower_path.ends_with(".bigbed") || lower_path.ends_with(".bb") {
        Ok(format!("-"))  // BigBed files have embedded index like BigWig
    } else if lower_path.ends_with(".vcf.gz")
        || lower_path.ends_with(".gff.gz")
        || lower_path.ends_with(".bed.gz")
        || lower_path.ends_with(".gtf.gz")
        || lower_path.ends_with(".bed")
        || lower_path.ends_with(".bedgraph.gz")
    {
        Ok(format!("{}.tbi", file_path))
    } else {
        Err(ExtensionError::IndexPathError(
            "Unable to get index path from file extension".into(),
        ))
    }
}

pub fn get_output_format(file_path: &str) -> Result<OutputFormat, ExtensionError> {
    let lower_path = file_path.to_ascii_lowercase();
    // Logic to determine the output format based on the file type
    if lower_path.ends_with(".bam") {
        Ok(OutputFormat::BAM)
    } else if lower_path.ends_with(".bigwig") || lower_path.ends_with(".bw") {
        Ok(OutputFormat::BIGWIG)
    } else if lower_path.ends_with(".bigbed") || lower_path.ends_with(".bb") {
        Ok(OutputFormat::BIGBED)
    } else if lower_path.ends_with(".vcf.gz") {
        Ok(OutputFormat::VCF)
    } else if lower_path.ends_with(".gff.gz") {
        Ok(OutputFormat::GFF)
    } else if lower_path.ends_with(".gtf.gz") {
        Ok(OutputFormat::GTF)
    } else if lower_path.ends_with(".bed.gz") {
        Ok(OutputFormat::BED)
    } else if lower_path.ends_with(".bedgraph.gz") {
        Ok(OutputFormat::BEDGRAPH)
    } else if lower_path.ends_with(".fa") || lower_path.ends_with(".fasta") {
        Ok(OutputFormat::FASTA)
    } else {
        Err(ExtensionError::PathTypeError(
            "Unable to determine file format from extension".into(),
        ))
    }
}

#[test]
fn test_parse_coordinates() {
    let (chr, start, end) = parse_coordinates("chr1:1000-2000").unwrap();
    assert_eq!(chr, "chr1");
    assert_eq!(start, 1000);
    assert_eq!(end, 2000);
}

#[test]
fn test_get_search_options_local() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let request = FileSearchRequest::new("./mock_data/NA12878.gatk.cnv.vcf.gz".into(), "chr1:1000-2000".into());
    let options = get_search_options(request).unwrap();
    assert_eq!(
        options.file_path,
        format!("file://{}/mock_data/NA12878.gatk.cnv.vcf.gz", manifest_dir)
    );
    assert_eq!(options.chromosome, "chr1");
    assert_eq!(options.begin, 1000);
    assert_eq!(options.end, 2000);
    assert_eq!(
        options.index_path,
        format!("file://{}/mock_data/NA12878.gatk.cnv.vcf.gz.tbi", manifest_dir)
    );
    assert_eq!(options.output_format, OutputFormat::VCF);

    let request = FileSearchRequest::new("./mock_data/NA12878.gatk.cnv.vcf.gz".into(), "chr1".into());
    let options = get_search_options(request).unwrap();
    assert_eq!(options.chromosome, "chr1");
    assert_eq!(options.begin, 1);
    assert_eq!(options.end, 249_250_621); // Assuming chr1 length from longest_possible_genome

    let request = FileSearchRequest::new("./mock_data/crap.gatk.cnv.vcf.gz".into(),"chr1".into());
    let options = get_search_options(request);
    assert!(options.is_err());
}

#[test]
fn test_format_file_path() {
    let path = format_file_path("./mock_data/NA12878.gatk.cnv.vcf.gz").unwrap();
    assert!(path.starts_with("file://"));
    let path = format_file_path("./fake/path.vcf.gz");
    assert!(path.is_err());

    let path = format_file_path("s3://bucket/file.vcf.gz").unwrap();
    assert_eq!(path, "s3://bucket/file.vcf.gz");
    let path = format_file_path("gs://bucket/file.vcf.gz").unwrap();
    assert_eq!(path, "gs://bucket/file.vcf.gz");
    let path = format_file_path("http://example.com/file.vcf.gz").unwrap();
    assert_eq!(path, "http://example.com/file.vcf.gz");
    let path = format_file_path("https://example.com/file.vcf.gz").unwrap();
    assert_eq!(path, "https://example.com/file.vcf.gz");
}

#[test]
fn test_get_index_path() {
    let index = get_index_path("s3://bucket/file.bam").unwrap();
    assert_eq!(index, "s3://bucket/file.bam.bai");
    let index = get_index_path("gs://bucket/file.fa").unwrap();
    assert_eq!(index, "gs://bucket/file.fa.fai");
    let index = get_index_path("http://example.com/file.vcf.gz").unwrap();
    assert_eq!(index, "http://example.com/file.vcf.gz.tbi");
    let index = get_index_path("https://example.com/file.bigwig").unwrap();
    assert_eq!(index, "-");
    let index = get_index_path("file.bed.gz").unwrap();
    assert_eq!(index, "file.bed.gz.tbi");
    let index = get_index_path("file.bed").unwrap();
    assert_eq!(index, "file.bed.tbi");
    let index = get_index_path("file.unknown");
    assert!(index.is_err());
}

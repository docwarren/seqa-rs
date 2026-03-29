use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{genome::{chr_index, chromosome_len, get_longest_possible_genome}, indexes::{bai::BaiIndex, bigwig::BigwigIndex, fai::FaiIndex, tabix::Tabix}, models::{bam_header::header::BamHeader, tabix_header::TabixHeader}, traits::feature::Feature};

use super::output_format::OutputFormat;

/// Specifies the format of the CIGAR string in BAM output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CigarFormat {
    /// Standard CIGAR string as stored in the BAM file (e.g., "10M1D5M")
    #[default]
    Standard,
    /// Merged CIGAR with mismatch and deletion bases included (e.g., "10M1DA5M1XG")
    Merged,
}

/// Configuration for a genomic region search.
///
/// Build a `SearchOptions` with [`SearchOptions::new`] and the setter methods, or set
/// fields directly.  Pass the finished value to
/// [`crate::services::search::SearchService::search_features`] or to one of the
/// format-specific search functions in [`crate::api`].
///
/// # Example
///
/// ```rust
/// use seqa_core::api::search_options::SearchOptions;
/// use seqa_core::api::output_format::OutputFormat;
///
/// let mut opts = SearchOptions::new()
///     .set_file_path("s3://my-bucket/sample.bam".into())
///     .set_index_path("s3://my-bucket/sample.bam.bai".into())
///     .set_output_format("bam")
///     .set_genome("hg38")
///     .set_coordinates("chr1:100000-200000");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// URI of the genomic data file (local `file://`, `s3://`, `az://`, `gs://`, `http(s)://`).
    pub file_path: String,
    /// URI of the companion index file.  Set to `"-"` for self-indexed formats
    /// (BigWig, BigBed).
    pub index_path: String,
    /// Chromosome / contig name (e.g. `"chr1"` or `"1"`).
    pub chromosome: String,
    /// Query begin position (0-based).
    pub begin: u32,
    /// Query end position (exclusive, 0-based half-open).
    pub end: u32,
    /// Optional reference genome build (e.g. `"hg38"`, `"hg19"`).
    /// When set, chromosome lengths are resolved from that build instead of
    /// the longest known length across all supported builds.
    pub genome: Option<String>,
    /// File format of [`SearchOptions::file_path`].
    pub output_format: OutputFormat,
    /// Whether to include the file header lines in [`crate::api::search_result::SearchResult::lines`].
    /// Defaults to `true`.
    pub include_header: bool,
    /// When `true`, return only header lines and skip data records.
    /// Defaults to `false`.
    pub header_only: bool,
    /// Controls how CIGAR strings are rendered in BAM/SAM output.
    pub cigar_format: CigarFormat,
    /// Pre-parsed BigWig index; supply to avoid re-downloading on repeated queries.
    pub bigwig_index: Option<BigwigIndex>,
    /// Pre-parsed BigBed index; supply to avoid re-downloading on repeated queries.
    /// BigBed shares the same index structure as BigWig.
    pub bigbed_index: Option<BigwigIndex>,
    /// Pre-parsed BAM index; supply to avoid re-downloading on repeated queries.
    pub bam_index: Option<BaiIndex>,
    /// Pre-parsed BAM header; supply to avoid re-downloading on repeated queries.
    pub bam_header: Option<BamHeader>,
    /// Pre-parsed tabix index; supply to avoid re-downloading on repeated queries.
    pub tabix_index: Option<Tabix>,
    /// Pre-parsed tabix header; supply to avoid re-downloading on repeated queries.
    pub tabix_header: Option<TabixHeader>,
    /// Pre-parsed FASTA index; supply to avoid re-downloading on repeated queries.
    pub fasta_index: Option<FaiIndex>,
    /// When `true`, skip reading from and writing to the local index cache.
    /// The index will always be fetched from the remote source.
    /// Defaults to `false`.
    pub no_cache: bool,
}

impl SearchOptions {
    /// Creates a `SearchOptions` with sensible defaults.
    ///
    /// All path/chromosome fields are empty strings; `include_header` is `true`,
    /// `header_only` is `false`, and `output_format` is [`OutputFormat::STRING`].
    pub fn new() -> Self {
        SearchOptions {
            file_path: String::new(),
            index_path: String::new(),
            chromosome: String::new(),
            begin: 0,
            end: 0,
            genome: None,
            output_format: OutputFormat::STRING, // Default output format
            include_header: true,
            header_only: false,
            cigar_format: CigarFormat::Standard,
            bigwig_index: None,
            bigbed_index: None,
            bam_header: None,
            bam_index: None,
            tabix_index: None,
            tabix_header: None,
            fasta_index: None,
            no_cache: false,
        }
    }

    /// Sets the CIGAR string rendering format for BAM output.
    pub fn set_cigar_format(&mut self, cigar_format: CigarFormat) -> Self {
        self.cigar_format = cigar_format;
        self.clone()
    }

    /// Sets the URI of the genomic data file.
    pub fn set_file_path(&mut self, file_path: &str) -> Self {
        self.file_path = file_path.to_string();
        self.clone()
    }

    /// Sets the URI of the companion index file.
    pub fn set_index_path(&mut self, index_path: &str) -> Self {
        self.index_path = index_path.to_string();
        self.clone()
    }

    /// Parses a coordinate string and updates `chromosome`, `begin`, and `end`.
    ///
    /// Accepted formats (commas in numbers are stripped):
    ///
    /// | Input | Result |
    /// |-------|--------|
    /// | `"chr1"` | full chromosome (begin=1, end=chromosome length) |
    /// | `"chr1:12000"` | single position (end = begin + 1) |
    /// | `"chr1:12000-15000"` | explicit range |
    /// | `"chr1:12,000-15,000"` | commas ignored |
    ///
    /// When no explicit end is given, the chromosome length is resolved from
    /// [`SearchOptions::genome`] if set, otherwise from the longest known length
    /// across all supported reference builds.
    pub fn set_coordinates(&mut self, coords: &str) -> Self {
        let string:String = coords.replace(",", "");
        let parts: Vec<&str> = string.split(':').collect();

        if parts.len() == 2 {
            // Format: chr:begin-end or chr:position
            let range: Vec<&str> = parts[1].split('-').collect();
            self.chromosome = parts[0].to_string();

            if range.len() == 2 {
                // Format: chr:begin-end
                self.begin = range[0].parse().unwrap_or(1);
                self.end = range[1].parse().unwrap_or(1);
            } else if range.len() == 1 {
                // Format: chr:position (single position)
                let position: u32 = range[0].parse().unwrap_or(1);
                self.begin = position;
                self.end = position + 1;
            }
        } else if parts.len() == 1 {
            // Format: chr (no coordinates provided)
            // Use full chromosome length
            self.chromosome = parts[0].to_string();
            self.begin = 1;

            // If genome is known, use that genome's chromosome length
            if let Some(ref genome_name) = self.genome {
                if let Some(chr_len) = chromosome_len(&self.chromosome, genome_name) {
                    self.end = chr_len;
                } else {
                    // Genome specified but chromosome not found - use longest
                    if let Some(index) = chr_index(&self.chromosome) {
                        let longest_genome = get_longest_possible_genome();
                        self.end = longest_genome[index];
                    }
                }
            } else {
                // No genome specified - use longest possible
                if let Some(index) = chr_index(&self.chromosome) {
                    let longest_genome = get_longest_possible_genome();
                    self.end = longest_genome[index];
                }
            }
        }
        self.clone()
    }

    /// Sets the chromosome/contig name directly without parsing a full coordinate string.
    pub fn set_chromosome(&mut self, chromosome: &str) -> Self {
        self.chromosome = chromosome.to_string();
        self.clone()
    }

    /// Sets the reference genome build (stored in lowercase).
    ///
    /// Supported values: `"hg19"`, `"hg38"`, `"grch37"`, `"grch38"`.
    /// Used by [`SearchOptions::set_coordinates`] to resolve chromosome lengths.
    pub fn set_genome(&mut self, genome: &str) -> Self {
        self.genome = Some(genome.to_lowercase());
        self.clone()
    }

    /// Sets the query begin position (0-based).
    pub fn set_begin(&mut self, begin: u32) -> Self {
        self.begin = begin;
        self.clone()
    }

    /// Sets the query end position (exclusive, 0-based half-open).
    pub fn set_end(&mut self, end: u32) -> Self {
        self.end = end;
        self.clone()
    }

    /// Sets the output format by name (case-insensitive).
    /// Falls back to [`OutputFormat::VCF`] if the name is unrecognised.
    pub fn set_output_format(&mut self, output_format: &str) -> Self {
        let format = output_format.to_lowercase();
        self.output_format = OutputFormat::from_str(&format).unwrap_or(OutputFormat::VCF);
        self.clone()
    }

    /// Controls whether file header lines are included in the result.
    pub fn set_include_header(&mut self, include_header: bool) -> Self {
        self.include_header = include_header;
        self.clone()
    }

    /// When `true`, only header lines are returned and data records are skipped.
    pub fn set_header_only(&mut self, header_only: bool) -> Self {
        self.header_only = header_only;
        self.clone()
    }

    /// When `true`, skip reading from and writing to the local index cache.
    pub fn set_no_cache(&mut self, no_cache: bool) -> Self {
        self.no_cache = no_cache;
        self.clone()
    }

}

impl Feature for SearchOptions {

    fn get_begin(&self) -> u32 {
        self.begin
    }

    fn get_end(&self) -> u32 {
        self.end
    }

    fn get_length(&self) -> u32 {
        self.end - self.begin + 1
    }

    fn get_id(&self) -> String {
        format!("{}:{}-{}", self.chromosome, self.begin, self.end)
    }

    fn coordinate_system(&self) -> crate::models::coordinates::CoordinateSystem {
        crate::models::coordinates::CoordinateSystem::OneBasedClosed
    }

    fn get_chromosome(&self) -> String {
        self.chromosome.clone()
    }
}

impl Display for SearchOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}-{}", self.chromosome, self.begin, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_coordinates_full_chromosome_no_genome() {
        // Test: chr12 with no genome specified -> use longest genome
        let mut options = SearchOptions::new();
        options.set_coordinates("chr12");

        assert_eq!(options.chromosome, "chr12");
        assert_eq!(options.begin, 1);
        // chr12 longest length is from HG19/GRCH37: 133851895
        assert_eq!(options.end, 133851895);
    }

    #[test]
    fn test_set_coordinates_full_chromosome_with_genome() {
        // Test: chr12 with hg38 specified
        let mut options = SearchOptions::new();
        options.set_genome("hg38");
        options.set_coordinates("chr12");

        assert_eq!(options.chromosome, "chr12");
        assert_eq!(options.begin, 1);
        // chr12 in HG38: 133275309
        assert_eq!(options.end, 133275309);
    }

    #[test]
    fn test_set_coordinates_full_chromosome_with_hg19() {
        // Test: chr1 with hg19 specified
        let mut options = SearchOptions::new();
        options.set_genome("hg19");
        options.set_coordinates("chr1");

        assert_eq!(options.chromosome, "chr1");
        assert_eq!(options.begin, 1);
        // chr1 in HG19: 249250621
        assert_eq!(options.end, 249250621);
    }

    #[test]
    fn test_set_coordinates_single_position() {
        // Test: chr1:12000 -> begin=12000, end=12001
        let mut options = SearchOptions::new();
        options.set_coordinates("chr1:12000");

        assert_eq!(options.chromosome, "chr1");
        assert_eq!(options.begin, 12000);
        assert_eq!(options.end, 12001);
    }

    #[test]
    fn test_set_coordinates_range() {
        // Test: chr1:12000-15000
        let mut options = SearchOptions::new();
        options.set_coordinates("chr1:12000-15000");

        assert_eq!(options.chromosome, "chr1");
        assert_eq!(options.begin, 12000);
        assert_eq!(options.end, 15000);
    }

    #[test]
    fn test_set_coordinates_with_commas() {
        // Test: chr1:12,000-15,000 (commas should be stripped)
        let mut options = SearchOptions::new();
        options.set_coordinates("chr1:12,000-15,000");

        assert_eq!(options.chromosome, "chr1");
        assert_eq!(options.begin, 12000);
        assert_eq!(options.end, 15000);
    }

    #[test]
    fn test_set_coordinates_numeric_chromosome() {
        // Test: 12 (no chr prefix) with hg38
        let mut options = SearchOptions::new();
        options.set_genome("hg38");
        options.set_coordinates("12");

        assert_eq!(options.chromosome, "12");
        assert_eq!(options.begin, 1);
        // chr12 in HG38: 133275309
        assert_eq!(options.end, 133275309);
    }

    #[test]
    fn test_set_coordinates_chrx_with_position() {
        // Test: chrX:1000000
        let mut options = SearchOptions::new();
        options.set_coordinates("chrX:1000000");

        assert_eq!(options.chromosome, "chrX");
        assert_eq!(options.begin, 1000000);
        assert_eq!(options.end, 1000001);
    }

    #[test]
    fn test_set_genome() {
        // Test: genome setter converts to lowercase
        let mut options = SearchOptions::new();
        options.set_genome("HG38");

        assert_eq!(options.genome, Some("hg38".to_string()));
    }

    #[test]
    fn test_no_cache_defaults_to_false() {
        let options = SearchOptions::new();
        assert_eq!(options.no_cache, false);
    }

    #[test]
    fn test_set_no_cache() {
        let mut options = SearchOptions::new();
        options = options.set_no_cache(true);
        assert_eq!(options.no_cache, true);
    }
}

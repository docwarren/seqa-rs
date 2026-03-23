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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub file_path: String,
    pub index_path: String,
    pub chromosome: String,
    pub begin: u32,
    pub end: u32,
    pub genome: Option<String>,
    pub output_format: OutputFormat,
    pub include_header: bool,
    pub header_only: bool,
    pub cigar_format: CigarFormat,
    pub bigwig_index: Option<BigwigIndex>,
    pub bigbed_index: Option<BigwigIndex>,  // BigBed uses same index structure as BigWig
    pub bam_index: Option<BaiIndex>,
    pub bam_header: Option<BamHeader>,
    pub tabix_index: Option<Tabix>,
    pub tabix_header: Option<TabixHeader>,
    pub fasta_index: Option<FaiIndex>
}

impl SearchOptions {
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
            fasta_index: None
        }
    }

    pub fn set_cigar_format(&mut self, cigar_format: CigarFormat) -> Self {
        self.cigar_format = cigar_format;
        self.clone()
    }

    pub fn set_file_path(&mut self, file_path: &str) -> Self {
        self.file_path = file_path.to_string();
        self.clone()
    }

    pub fn set_index_path(&mut self, index_path: &str) -> Self {
        self.index_path = index_path.to_string();
        self.clone()
    }

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

    pub fn set_chromosome(&mut self, chromosome: &str) -> Self {
        self.chromosome = chromosome.to_string();
        self.clone()
    }

    pub fn set_genome(&mut self, genome: &str) -> Self {
        self.genome = Some(genome.to_lowercase());
        self.clone()
    }

    pub fn set_begin(&mut self, begin: u32) -> Self {
        self.begin = begin;
        self.clone()
    }

    pub fn set_end(&mut self, end: u32) -> Self {
        self.end = end;
        self.clone()
    }

    pub fn set_output_format(&mut self, output_format: &str) -> Self {
        let format = output_format.to_lowercase();
        self.output_format = OutputFormat::from_str(&format).unwrap_or(OutputFormat::VCF);
        self.clone()
    }

    pub fn set_include_header(&mut self, include_header: bool) -> Self {
        self.include_header = include_header;
        self.clone()
    }

    pub fn set_header_only(&mut self, header_only: bool) -> Self {
        self.header_only = header_only;
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
}

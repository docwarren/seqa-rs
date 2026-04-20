use crate::api::bam_search::{ BamError };
use crate::api::fasta_search::FastaSearchError;
use crate::api::output_format::OutputFormat;
use crate::api::search_options::SearchOptions;
use crate::api::search_result::SearchResult;
use crate::api::tabix_search::TabixSearchError;
use crate::api::bigwig_search::BigwigError;
use crate::api::bigbed_search::BigbedError;
use crate::api::{bam_search, bigwig_search, fasta_search, tabix_search, bigbed_search};
use crate::utils::UtilError;
use thiserror::Error;

/// Unified error returned by [`SearchService::search_features`].
///
/// Each variant wraps the format-specific error from the underlying search function.
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Search Error: {0}")]
    String(String),

    #[error("BAM error occurred")]
    Bam(#[from] BamError),

    #[error("Fasta error occurred")]
    Fasta(#[from] FastaSearchError),

    #[error("Tabix error occurred")]
    Tabix(#[from] TabixSearchError),

    #[error("BigWig error occurred")]
    BigWig(#[from] BigwigError),

    #[error("BigBed error occurred")]
    BigBed(#[from] BigbedError),

    #[error("Utility error occurred")]
    Util(#[from] UtilError),
}

/// Entry point for format-agnostic genomic region queries.
///
/// `SearchService` dispatches to the appropriate format-specific search function
/// based on [`SearchOptions::output_format`].  It does not hold any state; all
/// methods are `async` free functions.
///
/// # Supported formats
///
/// | [`OutputFormat`] | Underlying function |
/// |------------------|---------------------|
/// | `BAM` | [`crate::api::bam_search::bam_search`] |
/// | `BIGWIG` | [`crate::api::bigwig_search::bigwig_search`] |
/// | `BIGBED` | [`crate::api::bigbed_search::bigbed_search`] |
/// | `VCF`, `BED`, `BEDGRAPH`, `GFF`, `GTF` | [`crate::api::tabix_search::tabix_search`] |
/// | `FASTA` | [`crate::api::fasta_search::fasta_search`] |
pub struct SearchService;
impl SearchService {
    /// Searches for features in a file based on the provided search options.
    ///
    /// # Arguments
    ///
    /// * `search_options` - A `SearchOptions` containing the file path, coordinates, and other
    ///   query parameters.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `SearchResult` with the matching lines or an error.
    pub async fn search_features(search_options: &SearchOptions) -> Result<SearchResult, SearchError> {
        let results: Result<SearchResult, SearchError> = match search_options.output_format {
            OutputFormat::BAM => Ok(bam_search::bam_search(&search_options).await?),
            OutputFormat::BIGWIG => Ok(bigwig_search::bigwig_search(&search_options).await?),
            OutputFormat::BIGBED => Ok(bigbed_search::bigbed_search(&search_options).await?),
            OutputFormat::VCF => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::BED => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::BEDGRAPH => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::GFF => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::GTF => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::FASTA => Ok(fasta_search::fasta_search(&search_options).await?),
            _ => Err(SearchError::String("Output format is not supported for file search".into())),
        };

        match results {
            Ok(mut search_result) => {
                search_result.lines = search_result.lines.iter().map(|line| line.clone()).collect();
                return Ok(search_result);
            }

            Err(e) => {
                return Err(SearchError::String(format!("Error searching file: {}", e)));
            }
        }
    }
}

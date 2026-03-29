pub mod models;

use seqa_core::api::bam_search::{ BamError };
use seqa_core::api::fasta_search::FastaSearchError;
use seqa_core::api::output_format::OutputFormat;
use seqa_core::api::search_result::SearchResult;
use seqa_core::api::tabix_search::TabixSearchError;
use seqa_core::api::bigwig_search::BigwigError;
use seqa_core::api::{bam_search, bigwig_search, fasta_search, tabix_search};
use seqa_core::models::FileSearchRequest;
use seqa_core::utils::{get_search_options, UtilError};
use thiserror::Error;

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

    #[error("Utility error occurred")]
    Util(#[from] UtilError),
}

pub struct SearchService;
impl SearchService {
    /// Searches for features in a file based on the provided search request.
    ///
    /// # Arguments
    ///
    /// * `search_request` - A `FileSearchRequest` containing the path and coordinates for the search.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of strings representing the search results or an error message.
    pub async fn search_features(search_request: FileSearchRequest) -> Result<Vec<String>, SearchError> {
        let mut search_options = get_search_options(search_request)?;

        search_options.include_header = true;
        search_options.no_cache = true;

        let results: Result<SearchResult, SearchError> = match search_options.output_format {
            OutputFormat::BAM => { 
                search_options.include_header = false;
                Ok(bam_search::bam_search(&search_options).await?)
            },
            OutputFormat::BIGWIG => Ok(bigwig_search::bigwig_search(&search_options).await?),
            OutputFormat::VCF => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::BED => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::BEDGRAPH => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::GFF => Ok(tabix_search::tabix_search(&search_options).await?),
            OutputFormat::FASTA => Ok(fasta_search::fasta_search(&search_options).await?),
            _ => Err(SearchError::String("Output format is not supported for file search".into())),
        };

        match results {
            Ok(search_result) => {
                let line_vec: Vec<String> = search_result.lines.iter().map(|line| line.clone()).collect();
                return Ok(line_vec);
            }

            Err(e) => {
                return Err(SearchError::String(format!("Error searching file: {}", e)));
            }
        }
    }
}

use serde::{Deserialize, Serialize};
use crate::indexes::bai::BaiIndex;
use crate::indexes::bigwig::BigwigIndex;
use crate::indexes::fai::FaiIndex;
use crate::indexes::tabix::Tabix;
use crate::models::bam_header::header::BamHeader;
use crate::models::tabix_header::TabixHeader;

/// A high-level search request combining a file URI with a genomic coordinate string.
///
/// Pass to [`crate::utils::get_search_options`] to obtain a fully resolved
/// [`crate::api::search_options::SearchOptions`].
///
/// The optional index/header fields allow callers to supply a previously parsed index
/// so that it is not re-downloaded for every query.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSearchRequest {
    /// URI of the genomic data file.
    pub path: String,
    /// Genomic coordinate string, e.g. `"chr1:100000-200000"` or `"chrX"`.
    pub coordinates: String,
    /// Pre-parsed BigWig index (avoids re-downloading on repeated queries).
    pub bigwig_index: Option<BigwigIndex>,
    /// Pre-parsed BigBed index.  BigBed shares the same index structure as BigWig.
    pub bigbed_index: Option<BigwigIndex>,
    /// Pre-parsed BAM index.
    pub bam_index: Option<BaiIndex>,
    /// Pre-parsed BAM header.
    pub bam_header: Option<BamHeader>,
    /// Pre-parsed tabix index.
    pub tabix_index: Option<Tabix>,
    /// Pre-parsed tabix header.
    pub tabix_header: Option<TabixHeader>,
    /// Pre-parsed FASTA index.
    pub fasta_index: Option<FaiIndex>,
}

impl FileSearchRequest {
    /// Creates a new `FileSearchRequest` with only the required `path` and `coordinates`
    /// fields set; all optional index caches default to `None`.
    pub fn new(path: String, coordinates: String) -> FileSearchRequest {
        FileSearchRequest {
            path: path,
            coordinates: coordinates,
            bigwig_index: None,
            bigbed_index: None,
            bam_index: None,
            bam_header: None,
            tabix_header: None,
            tabix_index: None,
            fasta_index: None
        }
    }
}
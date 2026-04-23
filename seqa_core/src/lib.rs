//! # seqa_core
//!
//! Core library for querying genomic files across cloud and local storage backends.
//!
//! Soma supports BAM, VCF, GFF, GTF, BED, BedGraph, BigWig, BigBed, and FASTA formats,
//! with automatic index resolution and cloud-agnostic file access via [`object_store`].
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use seqa_core::api::search_options::SearchOptions;
//! use seqa_core::stores::StoreService;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let opts = SearchOptions::new(
//!         "s3://my-bucket/sample.bam",
//!         "chr1:100000-200000",
//!     );
//!
//!     let store_service = StoreService::new();
//!     let result = store_service.search_features(&opts).await?;
//!     for line in &result.lines {
//!         println!("{}", line);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Coordinate system
//!
//! All coordinates are stored internally in **0-based half-open** `[begin, end)` form.
//! Use [`models::coordinates::CoordinateSystem`] conversions when working with VCF/GFF/GTF
//! data, which natively use 1-based closed coordinates.
//!
//! ## Storage backends
//!
//! [`stores::StoreService::from_uri`] auto-detects the backend from the URL scheme:
//! - `s3://` — AWS S3 (requires `AWS_*` environment variables)
//! - `az://` — Azure Blob Storage (requires `AZURE_*` environment variables)
//! - `gs://` — Google Cloud Storage (requires `GOOGLE_*` environment variables)
//! - `file://` or a bare path — local filesystem
//! - `http(s)://` — HTTP/HTTPS
//!
//! ## Feature flags
//!
//! - **`sqlite`** — Enables [`sqlite`] module for gene-symbol and cytoband queries via SQLite.

pub mod genome;
pub mod traits;
pub mod stores;
pub mod codecs;
pub mod indexes;
pub mod api;
pub mod models;
pub mod utils;
pub mod services;

#[cfg(feature = "sqlite")]
pub mod sqlite;
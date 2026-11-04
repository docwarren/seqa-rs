// Copyright 2026 Seqa23
//
// Author: Andrew Warren
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # seqa_core
//!
//! Core library for querying genomic files across cloud and local storage backends.
//!
//! Seqa supports BAM, VCF, GFF, GTF, BED, BedGraph, BigWig, BigBed, and FASTA formats,
//! with automatic index resolution and cloud-agnostic file access via [`object_store`].
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use seqa_core::api::search_options::SearchOptions;
//! use seqa_core::stores::StoreService;
//! use seqa_core::api::search::search_features;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // One or more regions; results come back in the order the regions are given.
//!     let opts = SearchOptions::new(
//!         "s3://my-bucket/sample.bam",
//!         &vec!["chr1:100000-200000".to_string(), "chr2:5000-6000".to_string()],
//!     )?;
//!
//!     let store_service = StoreService::new();
//!     let result = search_features(&store_service, &opts).await?;
//!     for line in &result.lines {
//!         println!("{}", line);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Coordinate system
//!
//! Query strings passed to [`api::search_options::SearchOptions::new`] are **1-based closed**,
//! e.g. `chr1:1-10` is the first ten bases. Every [`coordinate::Coordinate`] carries its
//! [`models::coordinate_system::CoordinateSystem`] and is converted to **0-based half-open**
//! before comparing, so coordinates from different systems can be compared directly:
//! ```rust
//! use seqa_core::coordinate::Coordinate;
//! use seqa_core::models::coordinate_system::CoordinateSystem;
//!
//! let coord1 = Coordinate::new("chr1:1-10", CoordinateSystem::OneBasedClosed).unwrap();
//! let coord2 = Coordinate::new("1:10-100", CoordinateSystem::ZeroBasedHalfOpen).unwrap();
//! assert!(!coord1.overlaps(&coord2))
//! ```
//!
//! ## Storage backends
//!
//! [`stores::StoreService`] picks the backend from the URL scheme and builds each client
//! the first time it is used, then reuses it for the same bucket or host:
//! - `s3://` — AWS S3 (credentials from `AWS_*` environment variables)
//! - `az://` — Azure Blob Storage (credentials from `AZURE_*` environment variables)
//! - `gs://` — Google Cloud Storage (credentials from `GOOGLE_*` environment variables)
//! - `file://` or a local path (`/`, `./`, `../`) — local filesystem
//! - `http(s)://` — HTTP/HTTPS
//!
//! ## Feature flags
//!
//! - **`sqlite`** — Enables the `sqlite` module for gene-symbol and cytoband queries via SQLite.

pub mod api;
pub mod bam;
pub mod bigwig;
pub mod codecs;
pub mod coordinate;
pub mod fasta;
pub mod genome;
pub mod indexes;
pub mod models;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod stores;
pub mod tabix;
pub mod traits;
pub mod utils;

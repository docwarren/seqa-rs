use std::time::Duration;

use moka::future::Cache;
use seqa_core::indexes::bai::BaiIndex;
use seqa_core::indexes::fai::FaiIndex;
use seqa_core::indexes::tabix::Tabix;
use seqa_core::models::bam_header::header::BamHeader;
use seqa_core::models::tabix_header::TabixHeader;

const IDLE_TTL: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct AppCache {
    pub bai: Cache<String, BaiIndex>,
    pub tabix: Cache<String, Tabix>,
    pub fai: Cache<String, FaiIndex>,
    pub bam_header: Cache<String, BamHeader>,
    pub tabix_header: Cache<String, TabixHeader>,
}

impl AppCache {
    pub fn new() -> Self {
        Self {
            bai: build(),
            tabix: build(),
            fai: build(),
            bam_header: build(),
            tabix_header: build(),
        }
    }
}

impl Default for AppCache {
    fn default() -> Self {
        Self::new()
    }
}

fn build<V: Clone + Send + Sync + 'static>() -> Cache<String, V> {
    Cache::builder().time_to_idle(IDLE_TTL).build()
}

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

//! The single-ontology, process-wide cache.

use std::future::Future;
use std::sync::{Arc, LazyLock, RwLock};

use crate::Error;
use crate::index::Ontology;
use crate::loader::{MAX_OBO_BYTES, load_from_store};
use crate::location;

/// An ontology together with the normalised location it was loaded from.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) key: String,
    pub(crate) ontology: Ontology,
}

/// Holds at most one loaded ontology.
#[derive(Debug, Default)]
pub(crate) struct OntologyCache {
    inner: RwLock<Option<Arc<Loaded>>>,
}

/// The cache used by the public API.
pub(crate) static GLOBAL: LazyLock<OntologyCache> = LazyLock::new(OntologyCache::new);

impl OntologyCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn current_key_is(&self, key: &str) -> bool {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().is_some_and(|l| l.key == key)
    }

    /// Loads `key` with `fetch` unless it is already the active key. The lock is not held
    /// while fetching; on success the new ontology replaces the old one, on failure the
    /// cache is left unchanged.
    pub(crate) async fn load_with<F, Fut>(&self, key: String, fetch: F) -> Result<(), Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Ontology, Error>>,
    {
        if self.current_key_is(&key) {
            return Ok(());
        }
        let ontology = fetch().await?;
        let new = Arc::new(Loaded { key, ontology });
        let old = {
            let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
            guard.replace(new)
        };
        drop(old);
        Ok(())
    }

    /// Normalises `location` and loads it from its object store.
    pub(crate) async fn load(&self, location: &str) -> Result<(), Error> {
        let url = location::normalise(location)?;
        let key = url.to_string();
        let label = key.clone();
        self.load_with(key, || async move {
            let (store, path) = location::open(&url)?;
            load_from_store(store.as_ref(), &path, &label, MAX_OBO_BYTES).await
        })
        .await
    }

    /// The active ontology, or `NotLoaded`.
    pub(crate) fn current(&self) -> Result<Arc<Loaded>, Error> {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        guard.clone().ok_or(Error::NotLoaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const MINI: &str = "./tests/fixtures/mini.obo";
    const MINI2: &str = "tests/fixtures/mini2.obo";

    fn has(cache: &OntologyCache, id: &str) -> bool {
        cache
            .current()
            .unwrap()
            .ontology
            .terms
            .iter()
            .any(|t| t.term.id == id)
    }

    #[test]
    fn empty_cache_is_not_loaded() {
        assert!(matches!(
            OntologyCache::new().current(),
            Err(Error::NotLoaded)
        ));
    }

    #[tokio::test]
    async fn same_key_is_a_no_op_without_fetch() {
        let cache = OntologyCache::new();
        let fetches = AtomicUsize::new(0);
        for _ in 0..2 {
            cache
                .load_with("k".into(), || async {
                    fetches.fetch_add(1, Ordering::SeqCst);
                    Ok(Ontology::default())
                })
                .await
                .unwrap();
        }
        assert_eq!(fetches.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn relative_and_file_url_forms_are_the_same_location() {
        let cache = OntologyCache::new();
        cache.load(MINI).await.unwrap();
        let first = cache.current().unwrap();
        let abs = url::Url::from_file_path(
            std::env::current_dir()
                .unwrap()
                .join("tests/fixtures/mini.obo"),
        )
        .unwrap();
        cache.load(abs.as_str()).await.unwrap();
        assert!(
            Arc::ptr_eq(&first, &cache.current().unwrap()),
            "reload replaced the ontology"
        );
        assert_eq!(first.key, abs.as_str());
    }

    #[tokio::test]
    async fn new_location_replaces_and_releases_old() {
        let cache = OntologyCache::new();
        cache.load(MINI).await.unwrap();
        let weak = Arc::downgrade(&cache.current().unwrap());
        cache.load(MINI2).await.unwrap();
        assert!(has(&cache, "MONDO:0001149"));
        assert!(!has(&cache, "HP:0000252"));
        assert!(weak.upgrade().is_none(), "old ontology still held");
    }

    #[tokio::test]
    async fn failed_load_keeps_previous_ontology() {
        let cache = OntologyCache::new();
        cache.load(MINI).await.unwrap();
        let before = cache.current().unwrap();
        let err = cache
            .load("./tests/fixtures/missing.obo")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NotFound(_)), "{err:?}");
        let err = cache.load("ftp://h/x.obo").await.unwrap_err();
        assert!(matches!(err, Error::InvalidLocation(_)), "{err:?}");
        assert!(Arc::ptr_eq(&before, &cache.current().unwrap()));
        assert!(has(&cache, "HP:0000252"));
    }
}

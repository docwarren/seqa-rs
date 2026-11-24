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

//! Fetching and parsing an OBO document from an object store.

use std::io::Cursor;

use object_store::{ObjectStore, ObjectStoreExt, path::Path};

use crate::Error;
use crate::index::Ontology;

/// Largest OBO object that will be loaded: 4 GB.
pub(crate) const MAX_OBO_BYTES: u64 = 4_294_967_296;

fn store_error(location: &str, e: object_store::Error) -> Error {
    match e {
        object_store::Error::NotFound { .. } => Error::NotFound(location.to_string()),
        source => Error::Store {
            location: location.to_string(),
            source,
        },
    }
}

/// Checks the object's size, downloads it, then parses and indexes it on a blocking thread.
/// `location` is only used in error messages.
pub(crate) async fn load_from_store(
    store: &dyn ObjectStore,
    path: &Path,
    location: &str,
    max_bytes: u64,
) -> Result<Ontology, Error> {
    let meta = store
        .head(path)
        .await
        .map_err(|e| store_error(location, e))?;
    if meta.size > max_bytes {
        return Err(Error::TooLarge {
            location: location.to_string(),
            size: meta.size,
        });
    }
    let result = store
        .get(path)
        .await
        .map_err(|e| store_error(location, e))?;
    if result.meta.size > max_bytes {
        return Err(Error::TooLarge {
            location: location.to_string(),
            size: result.meta.size,
        });
    }
    let bytes = result.bytes().await.map_err(|e| store_error(location, e))?;
    let label = location.to_string();
    tokio::task::spawn_blocking(move || {
        let doc = fastobo::from_reader(Cursor::new(bytes))
            .map_err(|e| Error::Parse(format!("{label}: {e}")))?;
        Ok(Ontology::from_doc(doc))
    })
    .await
    .map_err(|e| Error::Parse(format!("{location}: parser failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use crate::test_support::CountingStore;
    use object_store::{ObjectStoreExt, memory::InMemory, path::Path};

    const VALID: &str = "format-version: 1.2\n\n[Term]\nid: HP:0000001\nname: All\n";

    async fn store_with(content: &[u8]) -> (CountingStore<InMemory>, Path) {
        let store = CountingStore::new(InMemory::new());
        let path = Path::from("dir/hp.obo");
        store.put(&path, content.to_vec().into()).await.unwrap();
        (store, path)
    }

    #[test]
    fn max_obo_bytes_is_4_gib() {
        assert_eq!(MAX_OBO_BYTES, 4_294_967_296);
    }

    #[tokio::test]
    async fn oversized_object_is_rejected_without_download() {
        let (store, path) = store_with(VALID.as_bytes()).await;
        let max = VALID.len() as u64 - 1;
        let err = load_from_store(&store, &path, "mem:///dir/hp.obo", max)
            .await
            .unwrap_err();
        match err {
            Error::TooLarge { location, size } => {
                assert_eq!(location, "mem:///dir/hp.obo");
                assert_eq!(size, VALID.len() as u64);
            }
            e => panic!("unexpected {e:?}"),
        }
        assert_eq!(store.heads(), 1);
        assert_eq!(store.gets(), 0, "body must not be downloaded");
    }

    #[tokio::test]
    async fn size_reported_by_get_is_checked_before_reading_body() {
        let (mut store, path) = store_with(VALID.as_bytes()).await;
        let max = VALID.len() as u64;
        store.get_size = Some(max + 1);
        let err = load_from_store(&store, &path, "mem:///dir/hp.obo", max)
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::TooLarge { size, .. } if size == max + 1),
            "{err:?}"
        );
        assert_eq!(store.gets(), 1);
        assert_eq!(store.body_reads(), 0, "body must not be read");
    }

    #[tokio::test]
    async fn object_exactly_at_limit_is_accepted() {
        let (store, path) = store_with(VALID.as_bytes()).await;
        load_from_store(&store, &path, "mem:///dir/hp.obo", VALID.len() as u64)
            .await
            .unwrap();
        assert_eq!(store.gets(), 1);
    }

    #[tokio::test]
    async fn permission_error_is_store_error_naming_location() {
        let (mut store, path) = store_with(VALID.as_bytes()).await;
        store.deny = true;
        let err = load_from_store(&store, &path, "s3://bucket/private.obo", MAX_OBO_BYTES)
            .await
            .unwrap_err();
        assert!(
            matches!(&err, Error::Store { location, source: object_store::Error::PermissionDenied { .. } }
                if location == "s3://bucket/private.obo"),
            "{err:?}"
        );
        assert!(err.to_string().contains("s3://bucket/private.obo"), "{err}");
    }

    #[tokio::test]
    async fn missing_object_is_not_found() {
        let store = InMemory::new();
        let err = load_from_store(
            &store,
            &Path::from("nope.obo"),
            "mem:///nope.obo",
            MAX_OBO_BYTES,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(&err, Error::NotFound(l) if l == "mem:///nope.obo"),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn malformed_obo_is_parse_error() {
        let (store, path) = store_with(b"this is not [[ obo\n").await;
        let err = load_from_store(&store, &path, "mem:///dir/hp.obo", MAX_OBO_BYTES)
            .await
            .unwrap_err();
        match err {
            Error::Parse(msg) => {
                assert!(msg.starts_with("mem:///dir/hp.obo: "), "{msg}");
                assert!(msg.len() > "mem:///dir/hp.obo: ".len());
            }
            e => panic!("unexpected {e:?}"),
        }
    }
}

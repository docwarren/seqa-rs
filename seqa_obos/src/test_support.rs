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

//! Test helpers shared by unit tests.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use object_store::path::Path;
use object_store::{
    CopyOptions, GetOptions, GetResult, GetResultPayload, ListResult, MultipartUpload, ObjectMeta,
    ObjectStore, PutMultipartOptions, PutOptions, PutPayload, PutResult, Result,
};

/// Wraps a store and counts body downloads (`get_opts` calls that are not `head`) and
/// body chunks read. `get_size` overrides the size reported by `get` (not `head`).
#[derive(Debug)]
pub(crate) struct CountingStore<S> {
    pub(crate) inner: S,
    pub(crate) gets: AtomicUsize,
    pub(crate) heads: AtomicUsize,
    pub(crate) body_reads: Arc<AtomicUsize>,
    pub(crate) get_size: Option<u64>,
    /// When set, every read fails with `PermissionDenied`.
    pub(crate) deny: bool,
}

impl<S> CountingStore<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self {
            inner,
            gets: AtomicUsize::new(0),
            heads: AtomicUsize::new(0),
            body_reads: Arc::new(AtomicUsize::new(0)),
            get_size: None,
            deny: false,
        }
    }
    pub(crate) fn gets(&self) -> usize {
        self.gets.load(Ordering::SeqCst)
    }
    pub(crate) fn heads(&self) -> usize {
        self.heads.load(Ordering::SeqCst)
    }
    pub(crate) fn body_reads(&self) -> usize {
        self.body_reads.load(Ordering::SeqCst)
    }
}

impl<S: fmt::Display> fmt::Display for CountingStore<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Counting({})", self.inner)
    }
}

#[async_trait]
impl<S: ObjectStore> ObjectStore for CountingStore<S> {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        self.inner.put_opts(location, payload, opts).await
    }
    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> Result<Box<dyn MultipartUpload>> {
        self.inner.put_multipart_opts(location, opts).await
    }
    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        if self.deny {
            return Err(object_store::Error::PermissionDenied {
                path: location.to_string(),
                source: "access denied".into(),
            });
        }
        if options.head {
            self.heads.fetch_add(1, Ordering::SeqCst);
            return self.inner.get_opts(location, options).await;
        }
        self.gets.fetch_add(1, Ordering::SeqCst);
        let mut result = self.inner.get_opts(location, options).await?;
        if let Some(size) = self.get_size {
            result.meta.size = size;
        }
        if let GetResultPayload::Stream(stream) = result.payload {
            let reads = Arc::clone(&self.body_reads);
            result.payload = GetResultPayload::Stream(
                stream
                    .inspect(move |_| {
                        reads.fetch_add(1, Ordering::SeqCst);
                    })
                    .boxed(),
            );
        }
        Ok(result)
    }
    fn delete_stream(
        &self,
        locations: BoxStream<'static, Result<Path>>,
    ) -> BoxStream<'static, Result<Path>> {
        self.inner.delete_stream(locations)
    }
    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        self.inner.list(prefix)
    }
    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }
    async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()> {
        self.inner.copy_opts(from, to, options).await
    }
}

/// The `mini.obo` fixture, indexed.
pub(crate) fn mini() -> crate::index::Ontology {
    let doc = fastobo::from_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/mini.obo"
    ))
    .unwrap();
    crate::index::Ontology::from_doc(doc)
}

/// Indexes an inline OBO document (a header is prepended).
pub(crate) fn ontology(src: &str) -> crate::index::Ontology {
    crate::index::Ontology::from_doc(
        fastobo::from_str(format!("format-version: 1.2\n\n{src}")).unwrap(),
    )
}

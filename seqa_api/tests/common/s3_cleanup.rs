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

use object_store::aws::AmazonS3Builder;
use object_store::{ObjectStore, ObjectStoreScheme};
use url::Url;

/// Deletes the S3 object at a URI when dropped, so a test removes the object it
/// targets even when one of its assertions fails.
pub struct DeleteOnDrop {
    uri: String,
}

impl DeleteOnDrop {
    /// Guards the S3 object at `uri`.
    pub fn new(uri: &str) -> Self {
        DeleteOnDrop {
            uri: uri.to_string(),
        }
    }
}

impl Drop for DeleteOnDrop {
    fn drop(&mut self) {
        let uri = self.uri.clone();
        let deletion = std::thread::spawn(move || delete_s3_object_blocking(&uri));
        if deletion.join().is_err() {
            eprintln!("could not delete {}", self.uri);
        }
    }
}

fn delete_s3_object_blocking(uri: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(delete_s3_object(uri));
}

async fn delete_s3_object(uri: &str) {
    let url = Url::parse(uri).unwrap();
    let (_, path) = ObjectStoreScheme::parse(&url).unwrap();
    let store = AmazonS3Builder::from_env().with_url(uri).build().unwrap();
    store.delete(&path).await.unwrap();
}

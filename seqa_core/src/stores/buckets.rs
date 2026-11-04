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

//! Account-level bucket and container enumeration.
//!
//! The [`object_store`] crate cannot do this: its `ObjectStore` trait is
//! entirely object-scoped, and every builder binds to a single bucket via
//! `with_bucket_name` / `with_container_name`.  There is no `ListBuckets`.
//!
//! Rather than pull in three vendor SDKs (`aws-sdk-s3` alone costs ~60 crates
//! and a second credential chain), this module issues the account-level REST
//! calls directly and signs them with the authorizers `object_store` already
//! exports — `AwsAuthorizer`, `AzureAuthorizer` — reusing the exact credential
//! resolution the rest of the crate uses.

use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey, AwsAuthorizer};
use object_store::azure::{AzureAuthorizer, AzureConfigKey, MicrosoftAzureBuilder};
use object_store::client::{HttpClient, HttpRequest, HttpRequestBody};
use object_store::gcp::GoogleCloudStorageBuilder;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

use super::error::StoreError;

/// A bucket name is required to build any cloud store, but credentials are
/// account-scoped, so a throwaway name is enough to reach the credential
/// provider.  It is never contacted.
const PLACEHOLDER_BUCKET: &str = "seqa-bucket-enumeration-placeholder";

/// Fallback when no region is configured; S3's global endpoint redirects.
const DEFAULT_AWS_REGION: &str = "us-east-1";

/// The cloud backends whose buckets can be enumerated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudBackend {
    AmazonS3,
    GoogleCloudStorage,
    MicrosoftAzure,
}

impl CloudBackend {
    /// Parses the identifier used by the frontend's storage-backend dropdown.
    pub fn from_id(id: &str) -> Result<Self, StoreError> {
        match id.to_ascii_lowercase().as_str() {
            "s3" | "aws" | "amazons3" => Ok(Self::AmazonS3),
            "gcs" | "gs" | "google" => Ok(Self::GoogleCloudStorage),
            "azure" | "az" => Ok(Self::MicrosoftAzure),
            // `local` is deliberately absent: the local filesystem has no
            // buckets, and the caller should list a directory instead.
            other => Err(StoreError::ValidationError(format!(
                "Unknown cloud backend: {other}"
            ))),
        }
    }
}

/// Lists every bucket (Azure: container) visible to the configured credentials,
/// as fully-qualified URIs ready to pass to `list_directory`.
///
/// URIs rather than bare names because the shapes differ per provider: S3 and
/// GCS address a bucket directly (`s3://bucket`), while Azure nests a container
/// inside an account (`az://account/container`). Returning names would leave
/// every caller to rebuild that, and dropping the Azure account produces a URI
/// that cannot be listed.
///
/// Credentials come from the same environment variables the rest of the crate
/// uses — see [`super::store`].
///
/// # Errors
///
/// Returns [`StoreError`] when credentials are missing or invalid, the account
/// cannot be determined, or the provider returns a non-success response.
pub async fn list_buckets(backend: CloudBackend) -> Result<Vec<String>, StoreError> {
    match backend {
        CloudBackend::AmazonS3 => list_s3_buckets().await,
        CloudBackend::GoogleCloudStorage => list_gcs_buckets().await,
        CloudBackend::MicrosoftAzure => list_azure_containers().await,
    }
}

/// Sends a pre-signed request and returns the body, failing on non-2xx.
async fn send(request: HttpRequest) -> Result<Vec<u8>, StoreError> {
    let client = HttpClient::new(reqwest::Client::new());

    let response = client
        .execute(request)
        .await
        .map_err(|e| StoreError::ListError(format!("Bucket listing request failed: {e}")))?;

    let status = response.status();
    let body = response
        .into_body()
        .bytes()
        .await
        .map_err(|e| StoreError::ListError(format!("Could not read response body: {e}")))?;

    if !status.is_success() {
        return Err(StoreError::ListError(format!(
            "Bucket listing returned {status}: {}",
            String::from_utf8_lossy(&body)
        )));
    }

    Ok(body.to_vec())
}

fn build_get(url: &str) -> Result<HttpRequest, StoreError> {
    http::Request::builder()
        .method("GET")
        .uri(url)
        .body(HttpRequestBody::empty())
        .map_err(|e| StoreError::ValidationError(format!("Invalid request URL {url}: {e}")))
}

/// Collects the text of every `<Name>` element nested inside `container`.
///
/// Both the S3 and Azure listing documents wrap their entries in a container
/// element (`Buckets` / `Containers`) whose children each carry a `Name`.
/// Scoping to that parent avoids picking up unrelated `Name` elements such as
/// the `Owner` block in S3's response.
fn extract_names(xml: &[u8], container: &str) -> Result<Vec<String>, StoreError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut names = Vec::new();
    let mut depth_in_container = 0usize;
    let mut in_name = false;
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                let tag = local_name(element.name().as_ref());
                if tag == container {
                    depth_in_container += 1;
                } else if depth_in_container > 0 && tag == "Name" {
                    in_name = true;
                }
            }
            Ok(Event::End(element)) => {
                let tag = local_name(element.name().as_ref());
                if tag == container {
                    depth_in_container = depth_in_container.saturating_sub(1);
                } else if tag == "Name" {
                    in_name = false;
                }
            }
            Ok(Event::Text(text)) if in_name => {
                let value = text
                    .decode()
                    .map_err(|e| StoreError::ListError(format!("Malformed XML text: {e}")))?;
                names.push(value.to_string());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(StoreError::ListError(format!("Malformed XML: {e}"))),
            _ => {}
        }
        buffer.clear();
    }

    Ok(names)
}

/// Strips any namespace prefix from an XML tag.
fn local_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    match name.rsplit(':').next() {
        Some(local) => local.to_string(),
        None => name.to_string(),
    }
}

async fn list_s3_buckets() -> Result<Vec<String>, StoreError> {
    let builder = AmazonS3Builder::from_env().with_bucket_name(PLACEHOLDER_BUCKET);

    let region = builder
        .get_config_value(&AmazonS3ConfigKey::Region)
        .filter(|region| !region.is_empty())
        .unwrap_or_else(|| DEFAULT_AWS_REGION.to_string());

    let store = builder
        .build()
        .map_err(|e| StoreError::StoreNotInitialized(format!("{e}")))?;

    let credential = store
        .credentials()
        .get_credential()
        .await
        .map_err(|e| StoreError::StoreNotInitialized(format!("No AWS credentials: {e}")))?;

    let url = format!("https://s3.{region}.amazonaws.com/");
    let mut request = build_get(&url)?;

    AwsAuthorizer::new(&credential, "s3", &region).authorize(&mut request, None);

    let body = send(request).await?;

    Ok(extract_names(&body, "Buckets")?
        .into_iter()
        .map(|bucket| format!("s3://{bucket}"))
        .collect())
}

async fn list_azure_containers() -> Result<Vec<String>, StoreError> {
    let builder = MicrosoftAzureBuilder::from_env().with_container_name(PLACEHOLDER_BUCKET);

    // Read the account back off the builder rather than guessing an env var:
    // `from_env` understands AZURE_STORAGE_ACCOUNT_NAME and its aliases, and the
    // container-listing URL is built from the account name.
    let account = builder
        .get_config_value(&AzureConfigKey::AccountName)
        .filter(|account| !account.is_empty())
        .or_else(|| std::env::var("AZURE_STORAGE_ACCOUNT").ok())
        .ok_or_else(|| {
            StoreError::ValidationError(
                "AZURE_STORAGE_ACCOUNT_NAME must be set to list containers".to_string(),
            )
        })?;

    let store = builder
        .build()
        .map_err(|e| StoreError::StoreNotInitialized(format!("{e}")))?;

    let credential = store
        .credentials()
        .get_credential()
        .await
        .map_err(|e| StoreError::StoreNotInitialized(format!("No Azure credentials: {e}")))?;

    let url = format!("https://{account}.blob.core.windows.net/?comp=list");
    let mut request = build_get(&url)?;

    AzureAuthorizer::new(&credential, &account).authorize(&mut request);

    let body = send(request).await?;

    // Azure containers live inside an account, and the account must stay in the
    // URI or the result cannot be listed.
    Ok(extract_names(&body, "Containers")?
        .into_iter()
        .map(|container| format!("az://{account}/{container}"))
        .collect())
}

async fn list_gcs_buckets() -> Result<Vec<String>, StoreError> {
    let project_id = google_project_id()?;

    let store = GoogleCloudStorageBuilder::from_env()
        .with_bucket_name(PLACEHOLDER_BUCKET)
        .build()
        .map_err(|e| StoreError::StoreNotInitialized(format!("{e}")))?;

    let credential = store
        .credentials()
        .get_credential()
        .await
        .map_err(|e| StoreError::StoreNotInitialized(format!("No GCP credentials: {e}")))?;

    let url = format!("https://storage.googleapis.com/storage/v1/b?project={project_id}");
    let request = http::Request::builder()
        .method("GET")
        .uri(&url)
        .header("Authorization", format!("Bearer {}", credential.bearer))
        .body(HttpRequestBody::empty())
        .map_err(|e| StoreError::ValidationError(format!("Invalid request URL {url}: {e}")))?;

    let body = send(request).await?;

    let parsed: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| StoreError::ListError(format!("Malformed GCS response: {e}")))?;

    Ok(parsed
        .get("items")
        .and_then(|items| items.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name")?.as_str())
                .map(|bucket| format!("gs://{bucket}"))
                .collect()
        })
        .unwrap_or_default())
}

/// Resolves the GCP project ID, which `object_store` does not carry anywhere —
/// `ServiceAccountKey` is only an RSA keypair wrapper, and the bucket-listing
/// endpoint requires a project.
fn google_project_id() -> Result<String, StoreError> {
    if let Ok(project) = std::env::var("GOOGLE_PROJECT_ID")
        && !project.is_empty() {
            return Ok(project);
        }

    let service_account_json = match std::env::var("GOOGLE_SERVICE_ACCOUNT_KEY") {
        Ok(inline) if !inline.is_empty() => inline,
        _ => {
            let path = std::env::var("GOOGLE_SERVICE_ACCOUNT").map_err(|_| {
                StoreError::ValidationError(
                    "Set GOOGLE_PROJECT_ID, GOOGLE_SERVICE_ACCOUNT_KEY or \
                     GOOGLE_SERVICE_ACCOUNT to list GCS buckets"
                        .to_string(),
                )
            })?;
            std::fs::read_to_string(&path).map_err(|e| {
                StoreError::ValidationError(format!("Could not read {path}: {e}"))
            })?
        }
    };

    let parsed: serde_json::Value = serde_json::from_str(&service_account_json)
        .map_err(|e| StoreError::ValidationError(format!("Malformed service account key: {e}")))?;

    parsed
        .get("project_id")
        .and_then(|project| project.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            StoreError::ValidationError("Service account key has no project_id".to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_from_id() {
        assert_eq!(CloudBackend::from_id("s3").unwrap(), CloudBackend::AmazonS3);
        assert_eq!(
            CloudBackend::from_id("GCS").unwrap(),
            CloudBackend::GoogleCloudStorage
        );
        assert_eq!(
            CloudBackend::from_id("azure").unwrap(),
            CloudBackend::MicrosoftAzure
        );
        // Local has no buckets and must not be routed here.
        assert!(CloudBackend::from_id("local").is_err());
        assert!(CloudBackend::from_id("nonsense").is_err());
    }

    #[test]
    fn test_extract_names_from_s3_response() {
        // Note the Owner block: its DisplayName must not be collected, and
        // scoping to <Buckets> is what prevents that.
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
        <ListAllMyBucketsResult>
          <Owner><ID>abc123</ID><DisplayName>owner-name</DisplayName></Owner>
          <Buckets>
            <Bucket><Name>genomics-data</Name><CreationDate>2024-01-01T00:00:00Z</CreationDate></Bucket>
            <Bucket><Name>reference-files</Name><CreationDate>2024-02-01T00:00:00Z</CreationDate></Bucket>
          </Buckets>
        </ListAllMyBucketsResult>"#;

        let names = extract_names(xml, "Buckets").unwrap();
        assert_eq!(names, vec!["genomics-data", "reference-files"]);
    }

    #[test]
    fn test_extract_names_from_azure_response() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
        <EnumerationResults ServiceEndpoint="https://acct.blob.core.windows.net/">
          <Containers>
            <Container><Name>alignments</Name><Properties/></Container>
            <Container><Name>variants</Name><Properties/></Container>
          </Containers>
          <NextMarker/>
        </EnumerationResults>"#;

        let names = extract_names(xml, "Containers").unwrap();
        assert_eq!(names, vec!["alignments", "variants"]);
    }

    #[test]
    fn test_extract_names_handles_empty_listing() {
        let xml = br#"<ListAllMyBucketsResult><Buckets/></ListAllMyBucketsResult>"#;
        assert!(extract_names(xml, "Buckets").unwrap().is_empty());
    }

    #[test]
    fn test_local_name_strips_namespace() {
        assert_eq!(local_name(b"Name"), "Name");
        assert_eq!(local_name(b"ns:Name"), "Name");
    }
}

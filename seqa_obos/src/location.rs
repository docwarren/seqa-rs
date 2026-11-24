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

//! Turning a user-supplied location string into a normalised URL and an object store.

use object_store::{ObjectStore, path::Path};
use url::Url;

use crate::Error;

/// Normalises a location: URLs (scheme longer than one character) pass through,
/// anything else is treated as a filesystem path and becomes an absolute `file://` URL.
pub(crate) fn normalise(location: &str) -> Result<Url, Error> {
    if let Ok(url) = Url::parse(location)
        && url.scheme().len() > 1
    {
        return Ok(url);
    }
    let invalid = |reason: &str| Error::InvalidLocation(format!("{location}: {reason}"));
    let abs = std::path::absolute(location).map_err(|e| invalid(&e.to_string()))?;
    Url::from_file_path(&abs).map_err(|()| invalid("not a valid file path"))
}

/// Environment variable prefixes that may configure a store: the variables
/// `object_store`'s own `from_env()` builders read.
const PROVIDER_PREFIXES: [&str; 3] = ["AWS_", "AZURE_", "GOOGLE_"];

/// Builds the store options for `url` from `vars`: only variables whose names start with
/// `AWS_`, `AZURE_` or `GOOGLE_` are kept (any other variable could reconfigure the store
/// through `object_store`'s short key aliases), plus `allow_http=true` for `http://`.
pub(crate) fn store_options(
    url: &Url,
    vars: impl IntoIterator<Item = (String, String)>,
) -> Vec<(String, String)> {
    let mut options: Vec<(String, String)> = vars
        .into_iter()
        .filter(|(k, _)| PROVIDER_PREFIXES.iter().any(|p| k.starts_with(p)))
        .collect();
    if url.scheme() == "http" {
        options.push(("allow_http".into(), "true".into()));
    }
    options
}

/// Builds the object store for a normalised URL, configured from provider environment
/// variables (see [`store_options`]).
pub(crate) fn open(url: &Url) -> Result<(Box<dyn ObjectStore>, Path), Error> {
    object_store::parse_url_opts(url, store_options(url, std::env::vars()))
        .map_err(|e| Error::InvalidLocation(format!("{url}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    fn cwd_url(rel: &str) -> String {
        url::Url::from_file_path(std::env::current_dir().unwrap().join(rel))
            .unwrap()
            .to_string()
    }

    #[test]
    fn urls_pass_through() {
        for s in [
            "s3://b/k",
            "az://container/hp.obo",
            "gs://bucket/hp.obo",
            "https://example.org/hp.obo",
            "file:///tmp/hp.obo",
        ] {
            assert_eq!(normalise(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn relative_path_becomes_absolute_file_url() {
        assert_eq!(normalise("./x.obo").unwrap().as_str(), cwd_url("x.obo"));
        assert_eq!(normalise("x.obo").unwrap().as_str(), cwd_url("x.obo"));
    }

    #[test]
    fn absolute_path_becomes_file_url() {
        assert_eq!(
            normalise("/tmp/a b.obo").unwrap().as_str(),
            "file:///tmp/a%20b.obo"
        );
    }

    #[test]
    fn single_letter_scheme_is_a_path() {
        let url = normalise("c:/x.obo").unwrap();
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.as_str(), cwd_url("c:/x.obo"));
    }

    #[test]
    fn relative_and_absolute_forms_share_a_key() {
        let abs = cwd_url("hp.obo");
        assert_eq!(normalise("./hp.obo").unwrap(), normalise(&abs).unwrap());
    }

    #[test]
    fn opens_supported_schemes() {
        for (s, path) in [
            ("s3://b/k", "k"),
            ("gs://bucket/dir/hp.obo", "dir/hp.obo"),
            ("abfs://c@acct.dfs.core.windows.net/hp.obo", "hp.obo"),
            ("abfss://c@acct.dfs.core.windows.net/d/hp.obo", "d/hp.obo"),
            ("az://c@acct.dfs.core.windows.net/hp.obo", "hp.obo"),
            ("https://example.org/data/hp.obo", "data/hp.obo"),
            ("file:///tmp/a%20b.obo", "tmp/a b.obo"),
        ] {
            let url = normalise(s).unwrap();
            let (_, p) = open(&url).unwrap_or_else(|e| panic!("{s}: {e}"));
            assert_eq!(p.as_ref(), path, "{s}");
        }
    }

    #[test]
    fn azure_scheme_is_recognised() {
        // azure://<container>/<path> carries no account name, so building the store needs
        // AZURE_STORAGE_ACCOUNT_NAME from the environment; check the scheme is supported.
        let url = normalise("azure://container/hp.obo").unwrap();
        assert_eq!(url.as_str(), "azure://container/hp.obo");
        let (scheme, path) = object_store::ObjectStoreScheme::parse(&url).unwrap();
        assert_eq!(scheme, object_store::ObjectStoreScheme::MicrosoftAzure);
        assert_eq!(path.as_ref(), "hp.obo");
    }

    fn vars(keys: &[&str]) -> Vec<(String, String)> {
        keys.iter()
            .map(|k| (k.to_string(), "v".to_string()))
            .collect()
    }

    fn keys(opts: &[(String, String)]) -> Vec<&str> {
        opts.iter().map(|(k, _)| k.as_str()).collect()
    }

    #[test]
    fn only_provider_env_vars_become_store_options() {
        let env = vars(&[
            "TOKEN",
            "REGION",
            "ENDPOINT",
            "ALLOW_INVALID_CERTIFICATES",
            "AWS_REGION",
            "AZURE_STORAGE_ACCOUNT_NAME",
            "GOOGLE_SERVICE_ACCOUNT",
            "aws_lowercase",
            "HOME",
        ]);
        let s3 = url::Url::parse("s3://b/k").unwrap();
        assert_eq!(
            keys(&store_options(&s3, env.clone())),
            [
                "AWS_REGION",
                "AZURE_STORAGE_ACCOUNT_NAME",
                "GOOGLE_SERVICE_ACCOUNT"
            ]
        );
    }

    #[test]
    fn http_adds_allow_http_only_for_http() {
        let env = vars(&["AWS_REGION", "ALLOW_HTTP"]);
        let http = url::Url::parse("http://h/x.obo").unwrap();
        assert_eq!(
            store_options(&http, env.clone()),
            vec![
                ("AWS_REGION".to_string(), "v".to_string()),
                ("allow_http".to_string(), "true".to_string())
            ]
        );
        let https = url::Url::parse("https://h/x.obo").unwrap();
        assert_eq!(keys(&store_options(&https, env)), ["AWS_REGION"]);
    }

    #[test]
    fn unsupported_scheme_is_invalid_location() {
        let url = normalise("ftp://h/x").unwrap();
        match open(&url) {
            Err(Error::InvalidLocation(msg)) => assert!(msg.contains("ftp://h/x"), "{msg}"),
            Err(e) => panic!("unexpected error {e:?}"),
            Ok(_) => panic!("ftp should be rejected"),
        }
    }
}

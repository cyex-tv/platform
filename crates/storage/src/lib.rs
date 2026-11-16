//! Object storage (ADR 0003): Cloudflare R2 in production, an S3 server locally (Versity Gateway in dev and CI), all through the S3 API;
//! an in-memory store for tests.
//!
//! Key layout:
//! - `emotes/{sha256}/{variant}`: public, immutable, served at `cdn.cyex.tv`;
//! - `originals/{sha256}`: never public;
//! - `cosmetics/`: reserved.

pub mod sigv4;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use cyex_tv_shared::config::ConfigError;
use url::Url;

use crate::sigv4::{Credentials, EMPTY_SHA256};

/// `Cache-Control` for emote files: they never change under one key.
pub const IMMUTABLE: &str = "public, max-age=31536000, immutable";

pub fn emote_key(sha256_hex: &str, variant: &str) -> String {
    format!("emotes/{sha256_hex}/{variant}")
}

pub fn original_key(sha256_hex: &str) -> String {
    format!("originals/{sha256_hex}")
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage request failed: {0}")]
    Request(String),
    #[error("storage answered {status}: {body}")]
    Status { status: u16, body: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub bytes: Bytes,
    pub content_type: Option<String>,
    pub cache_control: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct Put<'a> {
    pub content_type: &'a str,
    pub cache_control: Option<&'a str>,
}

/// Where objects go. Cheap to clone.
#[derive(Clone)]
pub enum Store {
    S3(Arc<S3>),
    Memory(Arc<Mutex<HashMap<String, Object>>>),
}

impl Store {
    pub fn memory() -> Self {
        Self::Memory(Arc::default())
    }

    pub fn s3(cfg: S3Config) -> Result<Self, reqwest::Error> {
        Ok(Self::S3(Arc::new(S3::new(cfg)?)))
    }

    pub async fn put(&self, key: &str, body: Bytes, put: Put<'_>) -> Result<(), StorageError> {
        match self {
            Self::S3(s3) => s3.put(key, body, put).await,
            Self::Memory(map) => {
                lock(map).insert(
                    key.to_owned(),
                    Object {
                        bytes: body,
                        content_type: Some(put.content_type.to_owned()),
                        cache_control: put.cache_control.map(str::to_owned),
                    },
                );
                Ok(())
            }
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<Object>, StorageError> {
        match self {
            Self::S3(s3) => s3.get(key).await,
            Self::Memory(map) => Ok(lock(map).get(key).cloned()),
        }
    }

    pub async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        match self {
            Self::S3(s3) => s3.head(key).await,
            Self::Memory(map) => Ok(lock(map).contains_key(key)),
        }
    }

    /// Deleting what isn't there is fine.
    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        match self {
            Self::S3(s3) => s3.delete(key).await,
            Self::Memory(map) => {
                lock(map).remove(key);
                Ok(())
            }
        }
    }

    /// The keys held, sorted. Memory store only; for tests.
    pub fn keys(&self) -> Vec<String> {
        match self {
            Self::S3(_) => Vec::new(),
            Self::Memory(map) => {
                let mut keys: Vec<String> = lock(map).keys().cloned().collect();
                keys.sort();
                keys
            }
        }
    }
}

fn lock(
    map: &Mutex<HashMap<String, Object>>,
) -> std::sync::MutexGuard<'_, HashMap<String, Object>> {
    // A panic while holding the lock leaves the map usable; take it anyway.
    map.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// S3 settings from `CYEX_TV_S3_*`.
#[derive(Clone, Debug)]
pub struct S3Config {
    /// `https://<account>.r2.cloudflarestorage.com`, or `http://127.0.0.1:9000` for the dev S3 server.
    pub endpoint: Url,
    pub bucket: String,
    pub credentials: Credentials,
}

impl S3Config {
    /// `None` when no endpoint is set (tests and the api, which writes nothing yet).
    pub fn from_source(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, ConfigError> {
        let Some(endpoint) = get("CYEX_TV_S3_ENDPOINT").filter(|v| !v.trim().is_empty()) else {
            return Ok(None);
        };
        let endpoint = Url::parse(endpoint.trim()).map_err(|err| ConfigError::Invalid {
            name: "CYEX_TV_S3_ENDPOINT",
            reason: err.to_string(),
        })?;
        let need = |name: &'static str| {
            get(name)
                .filter(|v| !v.trim().is_empty())
                .map(|v| v.trim().to_owned())
                .ok_or(ConfigError::Missing(name))
        };
        Ok(Some(Self {
            endpoint,
            bucket: need("CYEX_TV_S3_BUCKET")?,
            credentials: Credentials {
                access_key_id: need("CYEX_TV_S3_ACCESS_KEY_ID")?,
                secret_access_key: need("CYEX_TV_S3_SECRET_ACCESS_KEY")?,
                // R2 takes `auto`; the dev server takes anything.
                region: get("CYEX_TV_S3_REGION").unwrap_or_else(|| "auto".to_owned()),
            },
        }))
    }
}

pub struct S3 {
    http: reqwest::Client,
    cfg: S3Config,
}

impl S3 {
    pub fn new(cfg: S3Config) -> Result<Self, reqwest::Error> {
        // Built without a crypto provider (see the workspace Cargo.toml); use ring.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { http, cfg })
    }

    /// The signed request for `key`, path-style (`{endpoint}/{bucket}/{key}`).
    fn request(
        &self,
        method: reqwest::Method,
        key: &str,
        payload_sha256: &str,
        extra: &[(&str, &str)],
    ) -> reqwest::RequestBuilder {
        let path = sigv4::object_path(&self.cfg.bucket, key);
        let mut url = self.cfg.endpoint.clone();
        url.set_path(&path);
        let host = match url.port() {
            Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
            None => url.host_str().unwrap_or_default().to_owned(),
        };
        let now = Utc::now();
        let mut headers = vec![
            ("host".to_owned(), host),
            ("x-amz-content-sha256".to_owned(), payload_sha256.to_owned()),
            (
                "x-amz-date".to_owned(),
                now.format("%Y%m%dT%H%M%SZ").to_string(),
            ),
        ];
        for (k, v) in extra {
            headers.push(((*k).to_owned(), (*v).to_owned()));
        }
        let auth = sigv4::authorization(
            &self.cfg.credentials,
            method.as_str(),
            &url,
            &path,
            &headers,
            payload_sha256,
            now,
        );
        let mut request = self.http.request(method, url).header("authorization", auth);
        for (k, v) in headers.iter().filter(|(k, _)| k != "host") {
            request = request.header(k, v);
        }
        request
    }

    async fn send(request: reqwest::RequestBuilder) -> Result<reqwest::Response, StorageError> {
        request
            .send()
            .await
            .map_err(|err| StorageError::Request(err.to_string()))
    }

    async fn failed(response: reqwest::Response) -> StorageError {
        let status = response.status().as_u16();
        let body: String = response
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(300)
            .collect();
        StorageError::Status { status, body }
    }

    /// Creates the bucket if it doesn't exist (local setup and tests; R2's bucket is made by the
    /// owner).
    pub async fn ensure_bucket(&self) -> Result<(), StorageError> {
        let path = format!("/{}", self.cfg.bucket);
        let mut url = self.cfg.endpoint.clone();
        url.set_path(&path);
        let host = match url.port() {
            Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
            None => url.host_str().unwrap_or_default().to_owned(),
        };
        let now = Utc::now();
        let headers = vec![
            ("host".to_owned(), host),
            ("x-amz-content-sha256".to_owned(), EMPTY_SHA256.to_owned()),
            (
                "x-amz-date".to_owned(),
                now.format("%Y%m%dT%H%M%SZ").to_string(),
            ),
        ];
        let auth = sigv4::authorization(
            &self.cfg.credentials,
            "PUT",
            &url,
            &path,
            &headers,
            EMPTY_SHA256,
            now,
        );
        let mut request = self.http.put(url).header("authorization", auth);
        for (k, v) in headers.iter().filter(|(k, _)| k != "host") {
            request = request.header(k, v);
        }
        let response = Self::send(request).await?;
        // 409: it exists (BucketAlreadyOwnedByYou).
        if response.status().is_success() || response.status() == reqwest::StatusCode::CONFLICT {
            Ok(())
        } else {
            Err(Self::failed(response).await)
        }
    }

    async fn put(&self, key: &str, body: Bytes, put: Put<'_>) -> Result<(), StorageError> {
        let hash = sigv4::sha256_hex(&body);
        let mut extra = vec![("content-type", put.content_type)];
        if let Some(cache) = put.cache_control {
            extra.push(("cache-control", cache));
        }
        let response = Self::send(
            self.request(reqwest::Method::PUT, key, &hash, &extra)
                .body(body),
        )
        .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(Self::failed(response).await)
        }
    }

    async fn get(&self, key: &str) -> Result<Option<Object>, StorageError> {
        let response =
            Self::send(self.request(reqwest::Method::GET, key, EMPTY_SHA256, &[])).await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(Self::failed(response).await);
        }
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let content_type = header("content-type");
        let cache_control = header("cache-control");
        let bytes = response
            .bytes()
            .await
            .map_err(|err| StorageError::Request(err.to_string()))?;
        Ok(Some(Object {
            bytes,
            content_type,
            cache_control,
        }))
    }

    async fn head(&self, key: &str) -> Result<bool, StorageError> {
        let response =
            Self::send(self.request(reqwest::Method::HEAD, key, EMPTY_SHA256, &[])).await?;
        match response.status() {
            s if s.is_success() => Ok(true),
            reqwest::StatusCode::NOT_FOUND => Ok(false),
            _ => Err(Self::failed(response).await),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let response =
            Self::send(self.request(reqwest::Method::DELETE, key, EMPTY_SHA256, &[])).await?;
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(())
        } else {
            Err(Self::failed(response).await)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn keys_follow_adr_0003() {
        assert_eq!(emote_key("ab12", "2x.webp"), "emotes/ab12/2x.webp");
        assert_eq!(original_key("ab12"), "originals/ab12");
    }

    #[tokio::test]
    async fn the_memory_store_behaves_like_a_bucket() {
        let store = Store::memory();
        assert_eq!(store.get("a").await.unwrap(), None);
        store
            .put(
                "a",
                Bytes::from_static(b"hi"),
                Put {
                    content_type: "image/webp",
                    cache_control: Some(IMMUTABLE),
                },
            )
            .await
            .unwrap();
        let got = store.get("a").await.unwrap().unwrap();
        assert_eq!(got.bytes, Bytes::from_static(b"hi"));
        assert_eq!(got.content_type.as_deref(), Some("image/webp"));
        assert!(store.exists("a").await.unwrap());
        store.delete("a").await.unwrap();
        store.delete("a").await.unwrap();
        assert!(!store.exists("a").await.unwrap());
    }

    fn load(pairs: &[(&str, &str)]) -> Result<Option<S3Config>, ConfigError> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        S3Config::from_source(|name| map.get(name).cloned())
    }

    #[test]
    fn s3_config_needs_every_part() {
        assert!(load(&[]).unwrap().is_none());
        let full = [
            ("CYEX_TV_S3_ENDPOINT", "http://127.0.0.1:9000"),
            ("CYEX_TV_S3_BUCKET", "cyex-tv-cdn"),
            ("CYEX_TV_S3_ACCESS_KEY_ID", "id"),
            ("CYEX_TV_S3_SECRET_ACCESS_KEY", "secret"),
        ];
        let cfg = load(&full).unwrap().unwrap();
        assert_eq!(cfg.credentials.region, "auto");
        assert_eq!(
            load(&full[..3]).unwrap_err(),
            ConfigError::Missing("CYEX_TV_S3_SECRET_ACCESS_KEY")
        );
    }
}

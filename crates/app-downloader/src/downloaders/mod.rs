use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use http::{
    header::{HeaderMap, IntoHeaderName},
    HeaderValue, Method,
};
use serde::{Deserialize, Serialize};

pub mod generic;
pub mod imgur;
pub mod instagram;
pub mod mastodon;
pub mod music;
pub mod reddit;
pub mod tiktok;
pub mod tumblr;
pub mod twitter;
pub mod yt_dlp;

#[async_trait::async_trait]
pub trait Downloader: Debug + Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String>;

    fn download(&self, req: &DownloadFileRequest) -> DownloaderReturn {
        let resolved = match self.get_resolved(req) {
            Ok(x) => x,
            Err(e) => {
                app_logger::error!("Failed to get links: {e}");
                return vec![Err(e)];
            }
        };

        self.download_resolved(&resolved)
    }

    fn download_resolved(&self, resolved_file: &ResolvedDownloadFileRequest) -> DownloaderReturn;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFileRequest {
    pub original_url: String,
    pub download_dir: PathBuf,
    #[serde(with = "http_serde::method", default = "default_get")]
    pub method: Method,
    #[serde(with = "http_serde::header_map", default)]
    pub headers: HeaderMap,
}

impl DownloadFileRequest {
    #[must_use]
    pub fn new(url: &str, download_dir: &Path) -> Self {
        Self {
            original_url: url.to_string(),
            download_dir: download_dir.to_path_buf(),
            method: Method::default(),
            headers: HeaderMap::default(),
        }
    }
}

const fn default_get() -> Method {
    Method::GET
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDownloadFileRequest {
    pub request_info: DownloadFileRequest,
    pub resolved_urls: Vec<DownloadUrlInfo>,
}

pub type ResolvedUrlHeaders = HeaderMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadUrlInfo {
    url: String,
    #[serde(with = "http_serde::header_map", default)]
    headers: ResolvedUrlHeaders,
}
impl DownloadUrlInfo {
    #[must_use]
    pub fn from_url(url: &str) -> Self {
        Self {
            url: url.to_string(),
            headers: HeaderMap::default(),
        }
    }

    #[must_use]
    pub fn with_headers(mut self, headers: ResolvedUrlHeaders) -> Self {
        self.headers = headers;
        self
    }

    #[must_use]
    pub fn with_header<K, V>(mut self, key: K, value: &V) -> Self
    where
        K: IntoHeaderName,
        V: ToString,
    {
        let value = value.to_string();
        if let Ok(value) = HeaderValue::from_str(&value) {
            self.headers.append(key, value);
        }
        self
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub const fn headers(&self) -> &ResolvedUrlHeaders {
        &self.headers
    }
}

impl From<&str> for DownloadUrlInfo {
    fn from(url: &str) -> Self {
        Self::from_url(url)
    }
}
impl From<String> for DownloadUrlInfo {
    fn from(url: String) -> Self {
        Self::from_url(&url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub request: DownloadFileRequest,
    pub path: PathBuf,
}

pub type DownloaderReturn = Vec<Result<DownloadResult, DownloaderError>>;

pub type DownloaderError = String;

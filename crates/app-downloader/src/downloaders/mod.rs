use std::{
    collections::HashMap,
    convert::Into,
    fmt::Debug,
    path::{Path, PathBuf},
    string::String,
};

use http::{
    header::{HeaderMap, IntoHeaderName},
    HeaderValue, Method,
};
use serde::{Deserialize, Serialize};

pub mod activity_pub;
pub mod generic;
pub mod imgur;
pub mod instagram;
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

        if let Some(preferred_downloader) = &resolved.prefer_downloader {
            return preferred_downloader.download_resolved(&resolved);
        }

        self.download_resolved(&resolved)
    }

    fn download_resolved(&self, resolved: &ResolvedDownloadFileRequest) -> DownloaderReturn;
}
impl std::fmt::Display for dyn Downloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Downloader::{}", self.name())
    }
}
impl Serialize for dyn Downloader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.to_string().serialize(serializer)
    }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolvedDownloadFileRequest {
    pub request_info: DownloadFileRequest,
    pub resolved_urls: Vec<DownloadUrlInfo>,
    #[serde(skip_deserializing)]
    pub prefer_downloader: Option<Box<dyn Downloader>>,
    pub download_options: HashMap<String, String>,
}
impl ResolvedDownloadFileRequest {
    #[must_use]
    fn new<U, I>(req: &DownloadFileRequest, urls: U) -> Self
    where
        U: IntoIterator<Item = I>,
        I: Into<DownloadUrlInfo>,
    {
        Self {
            request_info: req.clone(),
            resolved_urls: urls.into_iter().map(Into::into).collect(),
            prefer_downloader: None,
            download_options: HashMap::new(),
        }
    }

    pub fn from_urls<U, I>(req: &DownloadFileRequest, urls: U) -> Self
    where
        U: IntoIterator<Item = I>,
        I: Into<DownloadUrlInfo>,
    {
        Self::new(req, urls)
    }

    #[must_use]
    pub fn from_url<I>(req: &DownloadFileRequest, url: I) -> Self
    where
        I: Into<DownloadUrlInfo>,
    {
        Self::from_urls(req, [url])
    }

    #[must_use]
    pub fn with_preferred_downloader<D>(mut self, downloader: D) -> Self
    where
        D: Downloader + 'static,
    {
        self.prefer_downloader = Some(Box::new(downloader));
        self
    }

    #[must_use]
    pub fn with_download_option<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.download_options.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn download_option(&self, key: &str) -> Option<&str> {
        self.download_options.get(key).map(String::as_str)
    }

    #[must_use]
    pub fn download_option_parsed<T>(&self, key: &str) -> Option<T>
    where
        T: std::str::FromStr,
    {
        self.download_options.get(key).and_then(|x| x.parse().ok())
    }
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
impl From<&String> for DownloadUrlInfo {
    fn from(url: &String) -> Self {
        Self::from_url(url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub request: DownloadFileRequest,
    pub path: PathBuf,
}

pub type DownloaderReturn = Vec<Result<DownloadResult, DownloaderError>>;

pub type DownloaderError = String;

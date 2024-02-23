use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use http::{header::HeaderMap, Method};
use serde::{Deserialize, Serialize};

pub mod generic;
pub mod imgur;
pub mod instagram;
pub mod mastodon;
pub mod reddit;
pub mod tumblr;
pub mod twitter;
pub mod yt_dlp;

#[async_trait::async_trait]
pub trait Downloader: Debug + Send + Sync {
    fn name(&self) -> &'static str;

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
    pub resolved_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub request: DownloadFileRequest,
    pub path: PathBuf,
}

pub type DownloaderReturn = Vec<Result<DownloadResult, DownloaderError>>;

pub type DownloaderError = String;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    common::url::UrlWithMeta,
    downloaders::DownloaderEntry,
    extractors::{ExtractedInfo, ExtractedUrlInfo},
};

pub type DownloaderOptions = HashMap<String, serde_json::Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub url: UrlWithMeta,
    pub download_dir: PathBuf,
    #[serde(skip)]
    pub preferred_downloader: Option<DownloaderEntry>,
    pub downloader_options: DownloaderOptions,
}
impl DownloadRequest {
    #[must_use]
    pub fn from_url<U>(url: U, download_dir: &Path) -> Self
    where
        U: Into<UrlWithMeta>,
    {
        Self {
            url: url.into(),
            download_dir: download_dir.to_path_buf(),
            preferred_downloader: None,
            downloader_options: HashMap::new(),
        }
    }

    #[must_use]
    pub fn from_extracted_url(info: &ExtractedUrlInfo, download_dir: &Path) -> Self {
        Self {
            url: info.url.clone(),
            download_dir: download_dir.to_path_buf(),
            preferred_downloader: info.preferred_downloader.clone(),
            downloader_options: info.downloader_options.clone(),
        }
    }

    #[must_use]
    pub fn from_extracted_info(info: &ExtractedInfo, download_dir: &Path) -> Vec<Self> {
        info.urls
            .iter()
            .map(|x| Self::from_extracted_url(x, download_dir))
            .collect()
    }
}
impl DownloadRequest {
    #[must_use]
    pub const fn download_dir(&self) -> &PathBuf {
        &self.download_dir
    }

    #[must_use]
    pub fn with_preferred_downloader<D>(mut self, downloader: DownloaderEntry) -> Self {
        self.preferred_downloader = Some(downloader);
        self
    }

    #[must_use]
    pub fn with_downloader_option<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.downloader_options.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn downloader_option(&self, key: &str) -> Option<&serde_json::Value> {
        self.downloader_options.get(key)
    }
}

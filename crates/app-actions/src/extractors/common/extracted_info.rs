use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};

use super::extract_info_request::ExtractInfoRequest;
use crate::{
    common::url::UrlWithMeta,
    downloaders::{Downloader, DownloaderOptions},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedInfo {
    pub request: ExtractInfoRequest,
    pub urls: Vec<ExtractedUrlInfo>,
    pub meta: HashMap<String, serde_json::Value>,
}
impl ExtractedInfo {
    #[must_use]
    fn new<U, I>(req: &ExtractInfoRequest, urls: U) -> Self
    where
        U: IntoIterator<Item = I>,
        I: Into<ExtractedUrlInfo>,
    {
        Self {
            request: req.clone(),
            urls: urls.into_iter().map(Into::into).collect(),
            meta: HashMap::new(),
        }
    }

    pub fn from_urls<U, I>(req: &ExtractInfoRequest, urls: U) -> Self
    where
        U: IntoIterator<Item = I>,
        I: Into<ExtractedUrlInfo>,
    {
        Self::new(req, urls)
    }

    #[must_use]
    pub fn from_url<I>(req: &ExtractInfoRequest, url: I) -> Self
    where
        I: Into<ExtractedUrlInfo>,
    {
        Self::from_urls(req, [url])
    }

    #[must_use]
    pub fn with_preferred_downloader<D>(mut self, downloader: Option<D>) -> Self
    where
        D: Downloader + 'static,
    {
        let downloader = downloader.map(Arc::new);

        for x in &mut self.urls {
            #[allow(clippy::option_if_let_else)]
            {
                x.preferred_downloader = if let Some(downloader) = downloader.clone() {
                    Some(downloader)
                } else {
                    None
                }
            }
        }

        self
    }

    #[must_use]
    pub fn with_downloader_option<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        let key = key.into();
        let value = value.into();
        for x in &mut self.urls {
            x.downloader_options.insert(key.clone(), value.clone());
        }

        self
    }

    #[must_use]
    pub fn with_meta<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.meta.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn dedup_urls(mut self) -> Self {
        self.urls.dedup();
        self
    }
}

pub type PreferredDownloader = Arc<dyn Downloader>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedUrlInfo {
    pub url: UrlWithMeta,
    pub preferred_downloader: Option<PreferredDownloader>,
    pub downloader_options: DownloaderOptions,
}
impl ExtractedUrlInfo {
    #[must_use]
    pub fn new<U>(url: U) -> Self
    where
        U: Into<UrlWithMeta>,
    {
        Self {
            url: url.into(),
            preferred_downloader: None,
            downloader_options: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_preferred_downloader<D>(mut self, downloader: Option<D>) -> Self
    where
        D: Downloader + 'static,
    {
        if let Some(downloader) = downloader {
            self.preferred_downloader = Some(Arc::new(downloader));
        } else {
            self.preferred_downloader = None;
        }

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
    pub fn with_downloader_options<T>(mut self, options: T) -> Self
    where
        T: Into<DownloaderOptions>,
    {
        self.downloader_options = options.into();
        self
    }

    #[must_use]
    pub fn downloader_option(&self, key: &str) -> Option<&serde_json::Value> {
        self.downloader_options.get(key)
    }
}

impl From<UrlWithMeta> for ExtractedUrlInfo {
    fn from(url: UrlWithMeta) -> Self {
        Self::new(url)
    }
}

impl From<String> for ExtractedUrlInfo {
    fn from(url: String) -> Self {
        Self::new(url)
    }
}

impl From<&String> for ExtractedUrlInfo {
    fn from(url: &String) -> Self {
        Self::new(url)
    }
}

impl From<&str> for ExtractedUrlInfo {
    fn from(url: &str) -> Self {
        Self::new(url)
    }
}

impl PartialEq for ExtractedUrlInfo {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl PartialOrd for ExtractedUrlInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.url.partial_cmp(&other.url)
    }
}

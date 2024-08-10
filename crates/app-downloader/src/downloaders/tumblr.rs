use std::string::ToString;

use once_cell::sync::Lazy;
use regex::Regex;

use super::{
    twitter::TwitterDownloader, DownloadFileRequest, Downloader, ResolvedDownloadFileRequest,
};
use crate::DownloaderReturn;

static DOMAIN_MATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?P<subdomain>[^\-][a-zA-Z0-9\-]{0,30}[^\-])\.)?tumblr\.com")
        .expect("Invalid regex")
});

#[derive(Debug, Default)]
pub struct TumblrDownloader;

#[async_trait::async_trait]
impl Downloader for TumblrDownloader {
    fn name(&self) -> &'static str {
        "tumblr"
    }

    fn description(&self) -> &'static str {
        "Downloads images and videos from Tumblr and screenshots the post itself."
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        TwitterDownloader.get_resolved(req)
    }

    fn download_resolved(&self, resolved_file: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        TwitterDownloader.download_resolved(resolved_file)
    }
}

impl TumblrDownloader {
    pub fn is_post_url(url: &str) -> bool {
        let Some(domain) = Self::domain_from_url(url) else {
            return false;
        };

        DOMAIN_MATCH.is_match(&domain)
    }

    fn domain_from_url(url: &str) -> Option<String> {
        url::Url::parse(url)
            .ok()
            .and_then(|x| x.domain().map(ToString::to_string))
    }
}

#[must_use]
pub fn download(req: &DownloadFileRequest) -> DownloaderReturn {
    TumblrDownloader.download(req)
}

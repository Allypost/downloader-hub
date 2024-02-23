use once_cell::sync::Lazy;
use regex::Regex;

use super::{
    twitter::TwitterDownloader, DownloadFileRequest, Downloader, ResolvedDownloadFileRequest,
};
use crate::DownloaderReturn;

pub static URL_MATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://(www\.)?tumblr\.com/(?P<username>[^/]+)/(?P<post_id>[0-9]+)(/|/[^/]+)?")
        .expect("Invalid regex")
});

#[derive(Debug, Default)]
pub struct TumblrDownloader;

#[async_trait::async_trait]
impl Downloader for TumblrDownloader {
    fn name(&self) -> &'static str {
        "tumblr"
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
        URL_MATCH.is_match(url)
    }
}

#[must_use]
pub fn download(req: &DownloadFileRequest) -> DownloaderReturn {
    TumblrDownloader.download(req)
}

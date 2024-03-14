use std::path::Path;

use app_config::Config;
use app_logger::{debug, trace};
use once_cell::sync::Lazy;
use regex::Regex;
use url::{form_urlencoded, Url};

use super::{
    generic::GenericDownloader, yt_dlp::YtDlpDownloader, DownloadFileRequest, DownloadResult,
    Downloader, DownloaderError, ResolvedDownloadFileRequest,
};
use crate::DownloaderReturn;

pub static URL_MATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://(www\.)?twitter\.com/(?P<username>[^/]+)/status/(?P<status_id>[0-9]+)")
        .expect("Invalid regex")
});

pub static MEDIA_URL_MATCH: Lazy<Regex> = Lazy::new(|| {
    // https://pbs.twimg.com/media/FqPFEWYWYBQ5iG3?format=png&name=small
    Regex::new(r"^https?://pbs\.twimg\.com/media/").expect("Invalid regex")
});

#[derive(Debug, Default)]
pub struct TwitterDownloader;

#[async_trait::async_trait]
impl Downloader for TwitterDownloader {
    fn name(&self) -> &'static str {
        "twitter"
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        YtDlpDownloader.get_resolved(req).or_else(|_| {
            debug!(?req, "Failed to download with yt-dlp. Trying to screenshot",);

            let endpoint = &Config::global().endpoint.twitter_screenshot_base_url;
            let tweet_screenshot_url =
                format!("{}/{}", endpoint.trim_end_matches('/'), req.original_url);

            trace!("Tweet screenshot URL: {:?}", &tweet_screenshot_url);

            Ok(ResolvedDownloadFileRequest {
                request_info: req.clone(),
                resolved_urls: vec![tweet_screenshot_url],
            })
        })
    }

    fn download_resolved(&self, resolved: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        YtDlpDownloader.download_resolved(resolved)
    }
}

impl TwitterDownloader {
    pub fn download_media_url(
        &self,
        download_dir: &Path,
        twitter_media_url: &str,
    ) -> Result<DownloadResult, String> {
        let mut parsed = Url::parse(twitter_media_url)
            .map_err(|x| format!("Failed to parse twitter media URL: {x:?}"))?;

        let url_without_name = {
            let params = parsed.query_pairs().filter(|(key, _)| key != "name");
            let params = form_urlencoded::Serializer::new(String::new())
                .clear()
                .extend_pairs(params)
                .finish();

            parsed.set_query(Some(&params));

            parsed.to_string()
        };

        GenericDownloader.download_one(
            &DownloadFileRequest::new(twitter_media_url, download_dir),
            &url_without_name,
        )
    }

    pub fn screenshot_tweet(
        &self,
        download_dir: &Path,
        url: &str,
    ) -> Result<DownloadResult, DownloaderError> {
        debug!(?url, "Trying to screenshot tweet");

        let endpoint = &Config::global().endpoint.twitter_screenshot_base_url;
        let tweet_screenshot_url = format!("{}/{}", endpoint.trim_end_matches('/'), url);

        trace!(url = ?tweet_screenshot_url, "Tweet screenshot URL");

        GenericDownloader.download_one(
            &DownloadFileRequest::new(url, download_dir),
            &tweet_screenshot_url,
        )
    }

    pub fn is_post_url(url: &str) -> bool {
        URL_MATCH.is_match(url)
    }

    pub fn is_media_url(url: &str) -> bool {
        MEDIA_URL_MATCH.is_match(url)
    }
}

pub fn download(req: &DownloadFileRequest) -> DownloaderReturn {
    debug!(?req, "Trying to download tweet media");

    let yt_dlp_result = YtDlpDownloader.download(req);

    if let Some(Err(_)) = yt_dlp_result.first() {
        debug!("Failed to download with yt-dlp. Trying to screenshot...");
        vec![screenshot_tweet(&req.download_dir, &req.original_url)]
    } else {
        yt_dlp_result
    }
}

pub fn download_media_url(
    download_dir: &Path,
    twitter_media_url: &str,
) -> Result<DownloadResult, String> {
    TwitterDownloader.download_media_url(download_dir, twitter_media_url)
}

fn screenshot_tweet(download_dir: &Path, url: &str) -> Result<DownloadResult, DownloaderError> {
    TwitterDownloader.screenshot_tweet(download_dir, url)
}

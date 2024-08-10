use std::time::Duration;

use app_logger::{debug, trace, warn};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use url::Url;

use super::{
    generic::GenericDownloader, twitter::TwitterDownloader, DownloadFileRequest, Downloader,
    ResolvedDownloadFileRequest,
};
use crate::{common::request::Client, downloaders::DownloadUrlInfo, DownloaderReturn};

pub static IS_NUMBERS_ONLY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d+$").expect("Invalid regex"));

#[derive(Debug, Default)]
pub struct MastodonDownloader;

#[async_trait::async_trait]
impl Downloader for MastodonDownloader {
    fn name(&self) -> &'static str {
        "mastodon"
    }

    fn description(&self) -> &'static str {
        "Downloads images and videos from Mastodon toots and screenshots the toot itself."
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        let info = Self::get_mastodon_info(&req.original_url)?;

        trace!(?info, "Got mastodon info");

        let screenshot_url = TwitterDownloader.screenshot_tweet_url(&info.url);

        trace!(?screenshot_url, "Downloading screenshot from url");

        Ok(ResolvedDownloadFileRequest {
            request_info: req.clone(),
            resolved_urls: vec![DownloadUrlInfo::from_url(&screenshot_url)],
        })
    }

    fn download_resolved(&self, resolved: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        GenericDownloader.download_resolved(resolved)
    }
}

#[derive(Debug, Deserialize)]
pub struct MastodonAccount {
    pub id: String,
    pub username: String,
    pub acct: String,
    pub display_name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct TootInfo {
    pub id: String,
    pub url: String,
    pub account: MastodonAccount,
}

impl MastodonDownloader {
    #[app_logger::instrument]
    pub fn get_mastodon_info(toot_url: &str) -> Result<TootInfo, String> {
        trace!("Getting mastodon info");

        let url_parsed = Url::parse(toot_url).map_err(|e| format!("Invalid URL: {e}"))?;

        let toot_id = {
            let path = url_parsed.path().trim_end_matches('/');

            path.split('/')
                .last()
                .ok_or_else(|| format!("Failed to get ID from toot URL: {toot_url:?}"))?
        };

        trace!(?toot_id, "Extracted ID from URL");

        let mastodon_host = url_parsed
            .host_str()
            .ok_or_else(|| format!("Failed to get host from toot URL: {toot_url:?}"))?;

        let mastodon_api_url = format!("https://{mastodon_host}/api/v1/statuses/{toot_id}");

        debug!(url = ?mastodon_api_url, "Making request to instance for toot info");

        let resp = Client::base()?
            .get(mastodon_api_url)
            .timeout(Duration::from_secs(5))
            .send()
            .map_err(|e| format!("Failed to send request to mastodon API: {e:?}"))?;

        trace!(?resp, "Finished api request");

        resp.json::<TootInfo>()
            .map_err(|e| format!("Failed to parse response from mastodon API: {e:?}"))
    }

    #[must_use]
    pub fn is_mastodon_toot(maybe_toot_url: &str) -> bool {
        Self::get_mastodon_info(maybe_toot_url).is_ok()
    }
}

#[must_use]
pub fn is_mastodon_toot(toot_url: &str) -> bool {
    MastodonDownloader::is_mastodon_toot(toot_url)
}

use std::collections::HashMap;

use app_logger::{debug, trace};

use super::{
    yt_dlp::YtDlpDownloader, DownloadFileRequest, DownloadUrlInfo, Downloader, DownloaderReturn,
    ResolvedDownloadFileRequest,
};
use crate::common::{request::Client, USER_AGENT};

#[derive(Debug, Default)]
pub struct TiktokDownloader;

#[async_trait::async_trait]
impl Downloader for TiktokDownloader {
    fn name(&self) -> &'static str {
        "tiktok"
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        let urls = get_media_download_urls(req)?;

        Ok(ResolvedDownloadFileRequest {
            request_info: req.clone(),
            resolved_urls: urls,
        })
    }

    fn download_resolved(&self, resolved_file: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        YtDlpDownloader.download_resolved(resolved_file)
    }
}

impl TiktokDownloader {
    #[must_use]
    pub fn is_post_url(url: &str) -> bool {
        url.starts_with("https://www.tiktok.com/@")
    }
}

fn get_media_download_urls(req: &DownloadFileRequest) -> Result<Vec<DownloadUrlInfo>, String> {
    debug!("Getting media download urls for tiktok post");

    let resp = Client::from_download_request(req, &req.original_url)?
        .send()
        .map_err(|e| format!("Failed to send request to imgur: {:?}", e))?;
    trace!(?resp, "Got response from tiktok");

    let mut resp_cookies = HashMap::<String, String>::new();
    for cookie in resp.cookies() {
        resp_cookies.insert(cookie.name().to_string(), cookie.value().to_string());
    }
    trace!(?resp_cookies, "Got cookies from tiktok response");

    let csrf_token = resp_cookies
        .get("tt_chain_token")
        .ok_or_else(|| "Failed to get csrf token from response cookies".to_string())?;
    trace!(?csrf_token, "Got csrf token from response cookies");

    let resp_body = resp
        .text()
        .map_err(|e| format!("Failed to get response body: {:?}", e))?;
    debug!("Got response body from tiktok");
    trace!(?resp_body, "Response body");

    let dom = tl::parse(&resp_body, tl::ParserOptions::default())
        .map_err(|e| format!("Failed to parse response body: {:?}", e))?;
    trace!("Parsed response body as HTML");
    let parser = dom.parser();
    let data_el = dom
        .get_element_by_id("__UNIVERSAL_DATA_FOR_REHYDRATION__")
        .ok_or_else(|| {
            "Failed to find element with id __UNIVERSAL_DATA_FOR_REHYDRATION__ in response body"
                .to_string()
        })?
        .get(parser)
        .ok_or_else(|| {
            "Failed to get element with id __UNIVERSAL_DATA_FOR_REHYDRATION__ in response body"
                .to_string()
        })?;
    let data_el_text = data_el.inner_text(parser);
    let post_data = serde_json::from_str::<serde_json::Value>(&data_el_text).map_err(|e| {
        format!(
            "Failed to parse post data from element with id __UNIVERSAL_DATA_FOR_REHYDRATION__: \
             {:?}",
            e
        )
    })?;
    trace!(?post_data, "Got post data from response body");

    let video_data = post_data
        .get("__DEFAULT_SCOPE__")
        .and_then(|x| x.get("webapp.video-detail"))
        .and_then(|x| x.get("itemInfo"))
        .and_then(|x| x.get("itemStruct"))
        .ok_or_else(|| "Failed to get video data from post data".to_string())?;
    trace!(?video_data, "Got video data from post data");

    let video_url = video_data
        .get("video")
        .and_then(|x| x.get("playAddr"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| "Failed to get video url from video data".to_string())?;
    trace!(?video_url, "Got video url from video data");

    dbg!(Ok(vec![DownloadUrlInfo::from_url(video_url)
        .with_header("User-Agent", &USER_AGENT)
        .with_header("Referer", &req.original_url)
        .with_header(
            "Cookie",
            &format!("tt_chain_token={}", csrf_token),
        )]))
}

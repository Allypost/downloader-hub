use std::string::ToString;

use reqwest::blocking::Response;
use serde::Deserialize;

use super::{
    generic::GenericDownloader, DownloadFileRequest, Downloader, ResolvedDownloadFileRequest,
};
use crate::{common::request::Client, DownloaderReturn};

#[must_use]
pub fn is_imgur_direct_media_url(url: &str) -> bool {
    url.starts_with("https://i.imgur.com/")
}

#[must_use]
pub fn is_imgur_url(url: &str) -> bool {
    url.starts_with("https://imgur.com/") || url.starts_with("http://imgur.com/")
}

#[derive(Debug, Default)]
pub struct ImgurDownloader;

#[async_trait::async_trait]
impl Downloader for ImgurDownloader {
    fn name(&self) -> &'static str {
        "imgur"
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        let post_data = get_post_data(req)?;
        let urls = post_data
            .media
            .into_iter()
            .map(|x| x.url)
            .collect::<Vec<_>>();

        Ok(ResolvedDownloadFileRequest {
            request_info: req.clone(),
            resolved_urls: urls,
        })
    }

    fn download_resolved(&self, resolved: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(rayon::current_num_threads().min(4))
            .build();

        let thread_pool = match thread_pool {
            Ok(x) => x,
            Err(e) => {
                app_logger::error!("Failed to create thread pool: {:?}", e);

                return vec![Err(format!("Failed to create thread pool: {:?}", e))];
            }
        };

        thread_pool.install(|| GenericDownloader.download_resolved(resolved))
    }
}

impl ImgurDownloader {
    #[must_use]
    pub fn is_media_url(url: &str) -> bool {
        url.starts_with("https://i.imgur.com/")
    }

    #[must_use]
    pub fn is_post_url(url: &str) -> bool {
        url.starts_with("https://imgur.com/") || url.starts_with("http://imgur.com/")
    }
}

#[derive(Debug, Deserialize)]
struct ImgurPostData {
    pub media: Vec<ImgurPostMedia>,
}

#[derive(Debug, Deserialize)]
struct ImgurPostMedia {
    url: String,
}

fn get_post_data(req: &DownloadFileRequest) -> Result<ImgurPostData, String> {
    let resp = Client::from_download_request(req, &req.original_url)?
        .send()
        .and_then(Response::text)
        .map_err(|e| format!("Failed to send request to imgur: {:?}", e))?;

    app_logger::trace!("Got response from imgur");

    let dom = tl::parse(&resp, tl::ParserOptions::default())
        .map_err(|e| format!("Failed to parse html from imgur: {:?}", e))?;
    let parser = dom.parser();

    app_logger::trace!("Parsed html from imgur");

    let script_data = dom
        .query_selector("script")
        .expect("Failed parse query selector")
        .filter_map(|x| x.get(parser))
        .filter_map(|x| x.as_tag())
        .find_map(|x| {
            x.inner_text(parser)
                .trim()
                .strip_prefix("window.postDataJSON=")
                .map(ToString::to_string)
        })
        .ok_or_else(|| "Failed to get script data".to_string())?;

    app_logger::trace!(script_data, "Got script data from imgur");

    // The replace is required because Imgur improperly always escapes single quotes
    serde_json::from_str::<String>(&script_data.replace("\\'", "'"))
        .or_else(|_| serde_json::from_str::<String>(&script_data))
        .and_then(|x| serde_json::from_str::<ImgurPostData>(&x))
        .map_err(|e| format!("Failed to deserialize script data from imgur: {:?}", e))
}

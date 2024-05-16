use std::{result::Result, string};

use app_logger::{debug, trace};
use once_cell::sync::Lazy;
use regex::Regex;

use super::{
    generic::GenericDownloader, DownloadFileRequest, Downloader, ResolvedDownloadFileRequest,
};
use crate::{common::request::Client, DownloaderReturn};

pub static URL_MATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://(www\.)?instagram.com/p/(?P<post_id>[^/?]+)").expect("Invalid regex")
});

#[derive(Debug, Default)]
pub struct InstagramDownloader;

#[async_trait::async_trait]
impl Downloader for InstagramDownloader {
    fn name(&self) -> &'static str {
        "instagram"
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        let media_urls = get_media_urls(&req.original_url)?;

        Ok(ResolvedDownloadFileRequest {
            request_info: req.clone(),
            resolved_urls: media_urls.into_iter().collect::<Vec<_>>(),
        })
    }

    fn download_resolved(&self, resolved_file: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(1).build();

        let thread_pool = match thread_pool {
            Ok(x) => x,
            Err(e) => {
                app_logger::error!("Failed to create thread pool: {:?}", e);

                return vec![Err(format!("Failed to create thread pool: {:?}", e))];
            }
        };

        thread_pool.install(|| GenericDownloader.download_resolved(resolved_file))
    }
}

impl InstagramDownloader {
    pub fn is_post_url(url: &str) -> bool {
        URL_MATCH.is_match(url)
    }
}

pub fn get_media_urls(url: &str) -> Result<Vec<String>, String> {
    fn get_api_response(post_id: &str) -> Result<serde_json::Value, String> {
        let query_hash = "2efa04f61586458cef44441f474eee7c";
        let query_args = serde_json::json!({
            "shortcode": post_id,
            "child_comment_count": 0,
            "fetch_comment_count": 0,
            "parent_comment_count": 0,
            "has_threaded_comments": true,
        });
        trace!("Query args: {:?}", &query_args);
        let query_args =
            serde_json::to_string(&query_args).map_err(|_e| "Failed to stringify json")?;

        let api_url = format!(
            "https://www.instagram.com/graphql/query/?query_hash={}&variables={}",
            &query_hash, &query_args,
        );

        debug!("Fetching from instagram API url: {:?}", &api_url);

        Client::base()?
            .get(&api_url)
            .send()
            .map_err(|e| format!("Failed to send request to instagram API: {e:?}"))?
            .text()
            .map_err(|e| format!("Failed to parse response from instagram API: {e:?}"))
            .and_then(|res_text| {
                serde_json::from_str::<serde_json::Value>(&res_text)
                    .map_err(|e| format!("Failed to parse response from instagram API: {e:?}"))
            })
    }

    trace!("Fetching instagram media URLs for: {}", &url);

    let post_id = URL_MATCH
        .captures(url)
        .and_then(|x| x.name("post_id"))
        .map(|x| x.as_str())
        .ok_or_else(|| "URL is not a valid Instagram post".to_string())?;
    debug!("Instagram post ID: {:?}", &post_id);

    let json_response = get_api_response(post_id)?;
    let edges = json_response
        .get("data")
        .and_then(|x| x.get("shortcode_media"))
        .and_then(serde_json::Value::as_object)
        .ok_or("Failed to get edges from response")?;

    if !edges.contains_key("edge_sidecar_to_children") {
        let url = edges
            .get("video_url")
            .or_else(|| edges.get("display_url"))
            .and_then(serde_json::Value::as_str)
            .map(string::ToString::to_string)
            .ok_or("Failed to get `display_url' on edges")?;

        debug!("Fetched Instagram media and got single image");

        return Ok(vec![url]);
    }

    debug!("Fetched Instagram media and got multiple images");

    let edges = &edges["edge_sidecar_to_children"]["edges"]
        .as_array()
        .expect("Failed to get edges on response")[..]
        .to_vec();

    let urls = edges
        .iter()
        .filter_map(|entry| {
            let node = entry.get("node").and_then(serde_json::Value::as_object)?;

            if node.contains_key("video_url") {
                debug!("Found video in post: {}", node["id"]);
                return node.get("video_url").and_then(serde_json::Value::as_str);
            }

            debug!("Found image in post: {}", node["id"]);
            return node.get("display_url").and_then(serde_json::Value::as_str);
        })
        .map(string::ToString::to_string)
        .collect::<Vec<String>>();

    debug!("Found multiple Instagram media");
    Ok(urls)
}

use std::result::Result;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use tracing::{debug, trace};
use url::Url;

use super::{ExtractInfoRequest, Extractor};
use crate::{common::request::Client, extractors::ExtractedInfo};

#[derive(Debug, Default)]
pub struct InstagramExtractor;

#[async_trait::async_trait]
impl Extractor for InstagramExtractor {
    fn name(&self) -> &'static str {
        "instagram"
    }

    fn description(&self) -> &'static str {
        "Get images and videos from Instagram posts"
    }

    async fn can_handle(&self, request: &ExtractInfoRequest) -> bool {
        Self::is_post_url(&request.url)
    }

    async fn extract_info(&self, request: &ExtractInfoRequest) -> Result<ExtractedInfo, String> {
        let media_urls = get_media_urls(request.url.as_str()).await?;

        Ok(ExtractedInfo::from_urls(request, media_urls))
    }
}

static URL_MATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https?://(www\.)?instagram.com/(p|reel)/(?P<post_id>[^/?]+)")
        .expect("Invalid regex")
});

impl InstagramExtractor {
    pub fn is_post_url(url: &Url) -> bool {
        URL_MATCH.is_match(url.as_str())
    }
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
#[allow(clippy::enum_variant_names)]
enum InstagramXDTGraphMedia {
    XDTGraphVideo {
        video_url: String,
    },
    XDTGraphImage {
        display_url: String,
    },
    XDTGraphSidecar {
        edge_sidecar_to_children: XDTGraphEdges,
    },
}
impl InstagramXDTGraphMedia {
    fn get_media_urls(&self) -> Vec<String> {
        match self {
            Self::XDTGraphVideo { video_url } => vec![video_url.clone()],
            Self::XDTGraphImage { display_url } => vec![display_url.clone()],
            Self::XDTGraphSidecar {
                edge_sidecar_to_children: edges,
            } => edges.get_media_urls(),
        }
    }
}

#[derive(Deserialize)]
struct XDTGraphEdge {
    node: InstagramXDTGraphMedia,
}

#[derive(Deserialize)]
struct XDTGraphEdges {
    edges: Vec<XDTGraphEdge>,
}
impl XDTGraphEdges {
    fn get_media_urls(&self) -> Vec<String> {
        self.edges
            .iter()
            .flat_map(|x| x.node.get_media_urls())
            .collect()
    }
}

async fn get_media_urls(url: &str) -> Result<Vec<String>, String> {
    trace!("Fetching instagram media URLs for: {}", &url);

    let post_id = URL_MATCH
        .captures(url)
        .and_then(|x| x.name("post_id"))
        .map(|x| x.as_str())
        .ok_or_else(|| "URL is not a valid Instagram post".to_string())?;
    debug!("Instagram post ID: {:?}", &post_id);

    get_api_response(post_id).await.map(|x| x.get_media_urls())
}

async fn get_api_response(post_id: &str) -> Result<InstagramXDTGraphMedia, String> {
    let query_variables = serde_json::json!({
        "shortcode": post_id,
        "fetch_comment_count": 0,
        "parent_comment_count": 0,
        "child_comment_count": 0,
        "fetch_like_count": 0,
        "fetch_tagged_user_count": null,
        "fetch_preview_comment_count": 2,
        "has_threaded_comments": true,
        "hoisted_comment_id": null,
        "hoisted_reply_id": null,
    });
    trace!("GraphQL Query Variables: {:?}", &query_variables);
    let query_variables_str =
        serde_json::to_string(&query_variables).map_err(|_e| "Failed to stringify json")?;

    let graphql_variables = {
        let mut q = form_urlencoded::Serializer::new(String::new());

        q.append_pair("variables", &query_variables_str);
        q.append_pair("server_timestamps", "true");
        q.append_pair("doc_id", "25531498899829322");

        q.finish()
    };
    trace!("GraphQL Variables: {:?}", &graphql_variables);

    let resp = Client::base()?
        .post("https://www.instagram.com/graphql/query/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(graphql_variables)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to instagram API: {e:?}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse response from instagram API: {e:?}"))?;

    trace!("Got response: {:?}", &resp);

    resp.get("data")
        .and_then(|x| x.get("xdt_shortcode_media"))
        .and_then(|x| serde_json::from_value::<InstagramXDTGraphMedia>(x.clone()).ok())
        .ok_or_else(|| "Failed to parse media from response".to_string())
}

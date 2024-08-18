use std::iter::Iterator;

use app_logger::{debug, trace};
use serde::Deserialize;
use url::Url;

use super::{node_info::NodeInfo, APHandler, HandleResult};
use crate::common::request::Client;

#[derive(Debug)]
pub struct MastodonHandler;

#[async_trait::async_trait]
impl APHandler for MastodonHandler {
    fn can_handle(&self, info: &NodeInfo, _url: &str) -> bool {
        matches!(info.software.name.to_lowercase().as_str(), "mastodon")
    }

    #[tracing::instrument]
    async fn handle(&self, info: &NodeInfo, url: &str) -> Result<HandleResult, String> {
        let parsed_url = Url::parse(url).map_err(|e| e.to_string())?;

        let toot_id = parsed_url
            .path_segments()
            .and_then(Iterator::last)
            .unwrap_or_default();

        let toot_info = TootInfo::from_id(&parsed_url, toot_id).await?;

        debug!(?toot_info, "Got toot info");

        let toot_url = &toot_info.url;
        if toot_url.host_str() != parsed_url.host_str() {
            return Ok(HandleResult::Delegated {
                url: toot_url.to_string(),
            });
        }

        Ok(HandleResult::Handled(
            toot_info
                .media_urls()
                .into_iter()
                .map(|x| x.to_string().into())
                .collect(),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct TootInfo {
    url: Url,
    media_attachments: Vec<MediaAttachment>,
}
impl TootInfo {
    #[tracing::instrument(skip(url))]
    async fn from_id(url: &Url, id: &str) -> Result<Self, String> {
        let api_url = {
            let mut url = url.clone();

            url.set_path(&format!("/api/v1/statuses/{}", id));
            url.query_pairs_mut().clear();

            url
        };
        trace!(?api_url, ?id, "Getting toot info");

        Client::base()?
            .get(api_url.as_str())
            .send()
            .await
            .map_err(|e| format!("Failed to get toot info: {:?}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse toot info: {:?}", e))
    }

    fn media_urls(&self) -> Vec<Url> {
        self.media_attachments
            .iter()
            .map(|x| x.url.clone())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct MediaAttachment {
    url: Url,
}

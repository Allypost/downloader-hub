use std::{collections::HashMap, iter::Iterator};

use app_logger::{debug, trace};
use serde::Deserialize;
use url::Url;

use super::{node_info::NodeInfo, HandleResult, Handler};
use crate::common::request::Client;

#[derive(Debug)]
pub struct MisskeyHandler;

impl Handler for MisskeyHandler {
    fn can_handle(&self, info: &NodeInfo, _url: &str) -> bool {
        matches!(
            info.software.name.to_lowercase().as_str(),
            "misskey" | "sharkey"
        )
    }

    #[tracing::instrument]
    fn handle(&self, info: &NodeInfo, url: &str) -> Result<HandleResult, String> {
        let parsed_url = Url::parse(url).map_err(|e| e.to_string())?;

        let post_id = parsed_url
            .path_segments()
            .and_then(Iterator::last)
            .unwrap_or_default();

        trace!(?post_id, "Got post ID");

        let post_info = PostInfo::from_id(&parsed_url, post_id)?;

        debug!(?post_info, "Got post info");

        Ok(HandleResult::Handled(
            post_info
                .media_urls()
                .into_iter()
                .map(|x| x.to_string().into())
                .collect(),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct PostInfo {
    files: Vec<PostFile>,
}
impl PostInfo {
    #[tracing::instrument(skip(url))]
    fn from_id(url: &Url, id: &str) -> Result<Self, String> {
        let api_url = {
            let mut url = url.clone();

            url.set_path("/api/notes/show");
            url.query_pairs_mut().clear();

            url
        };
        trace!(?api_url, ?id, "Getting post info");

        let body: HashMap<_, _> = HashMap::from_iter([("noteId", id)]);

        Client::base()?
            .post(api_url)
            .json(&body)
            .send()
            .map_err(|e| format!("Failed to get toot info: {:?}", e))?
            .json()
            .map_err(|e| format!("Failed to parse toot info: {:?}", e))
    }

    fn media_urls(&self) -> Vec<Url> {
        self.files.iter().map(|x| x.url.clone()).collect()
    }
}

#[derive(Debug, Deserialize)]
struct PostFile {
    url: Url,
}

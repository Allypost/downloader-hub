mod mastodon;
mod misskey;
mod node_info;

use app_logger::{debug, trace, warn};
pub use node_info::get_node_info;
use node_info::NodeInfo;
use once_cell::sync::Lazy;

use crate::downloaders::DownloadUrlInfo;

static HANDLERS: Lazy<Vec<Box<dyn Handler>>> = Lazy::new(handlers);

#[tracing::instrument]
pub fn get_post_media(post_url: &str) -> Result<Vec<DownloadUrlInfo>, String> {
    let mut maybe_post_url = post_url.to_string();
    let mut seen_urls = vec![];

    'outer: while seen_urls.len() < 10 {
        debug!(url = ?maybe_post_url, "Handling URL");
        if seen_urls.contains(&maybe_post_url) {
            return Err(format!(
                "URL loop detected ({}). Aborting.",
                seen_urls.join(" -> ")
            ));
        }
        seen_urls.push(maybe_post_url.clone());

        let info = get_node_info(&maybe_post_url)?;
        trace!(?info, "Got node info");

        for handler in HANDLERS.iter() {
            if !handler.can_handle(&info, &maybe_post_url) {
                continue;
            }

            trace!(?handler, "Handling with handler");

            let result = match handler.handle(&info, &maybe_post_url) {
                Ok(result) => result,
                Err(e) => {
                    warn!(?e, "Failed to handle URL");
                    continue;
                }
            };

            trace!(?result, "Got result");

            match result {
                HandleResult::Handled(urls) => {
                    trace!(?urls, "Got URLs");
                    return Ok(urls);
                }
                HandleResult::Delegated { url } => {
                    debug!(?url, "Delegating to another handler");
                    maybe_post_url = url;
                    continue 'outer;
                }
            }
        }

        return Err(format!(
            "No handler found for {:?} on {} version {}",
            maybe_post_url, info.software.name, info.software.version
        ));
    }

    Err(format!("No handler found for {:?}", maybe_post_url))
}

fn handlers() -> Vec<Box<dyn Handler>> {
    vec![
        Box::new(mastodon::MastodonHandler),
        Box::new(misskey::MisskeyHandler),
    ]
}

trait Handler: std::fmt::Debug + Send + Sync {
    fn handle(&self, info: &NodeInfo, url: &str) -> Result<HandleResult, String>;

    fn can_handle(&self, info: &NodeInfo, url: &str) -> bool;
}

#[derive(Debug)]
enum HandleResult {
    Handled(Vec<DownloadUrlInfo>),
    Delegated { url: String },
}

mod handlers;

use app_logger::{debug, trace};
use handlers::get_node_info;

use super::{
    generic::GenericDownloader, twitter::TwitterDownloader, DownloadFileRequest, Downloader,
    DownloaderReturn, ResolvedDownloadFileRequest,
};

#[derive(Debug, Default)]
pub struct ActivityPubDownloader;

impl Downloader for ActivityPubDownloader {
    fn name(&self) -> &'static str {
        "activity-pub"
    }

    fn description(&self) -> &'static str {
        "Downloads images and videos from many ActivityPub instances. Also screenshots the \
         tweet/post."
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        let mut urls = handlers::get_post_media(&req.original_url)?;
        urls.push(
            TwitterDownloader
                .screenshot_tweet_url(&req.original_url)
                .into(),
        );

        Ok(ResolvedDownloadFileRequest::from_urls(req, urls))
    }

    fn download_resolved(&self, resolved: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        GenericDownloader.download_resolved(resolved)
    }
}

impl ActivityPubDownloader {
    #[must_use]
    pub fn is_post_url(url: &str) -> bool {
        let resp = get_node_info(url);

        trace!(?resp, "Got node info response");

        if let Ok(resp) = &resp {
            debug!(?resp, "Got node info");
        }

        resp.is_ok()
    }
}

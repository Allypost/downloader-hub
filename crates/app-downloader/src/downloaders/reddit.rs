use super::{
    generic::GenericDownloader, DownloadFileRequest, Downloader, DownloaderReturn,
    ResolvedDownloadFileRequest,
};

#[must_use]
pub fn is_reddit_image_url(url: &str) -> bool {
    url.starts_with("https://i.redd.it/")
}

#[derive(Debug, Default)]
pub struct RedditDownloader;

#[async_trait::async_trait]
impl Downloader for RedditDownloader {
    fn name(&self) -> &'static str {
        "reddit"
    }

    fn description(&self) -> &'static str {
        "Downloads reddit media. Only works on media links (eg. https://i.redd.it/...)"
    }

    fn get_resolved<'a>(
        &'a self,
        req: &'a DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        GenericDownloader.get_resolved(req)
    }

    fn download_resolved<'a>(
        &'a self,
        resolved: &'a ResolvedDownloadFileRequest,
    ) -> DownloaderReturn {
        GenericDownloader.download_resolved(resolved)
    }
}

impl RedditDownloader {
    #[must_use]
    pub fn is_media_url(url: &str) -> bool {
        url.starts_with("https://i.redd.it/")
    }
}

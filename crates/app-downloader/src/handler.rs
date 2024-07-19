use once_cell::sync::Lazy;

use crate::{
    downloaders::{
        generic::GenericDownloader, imgur::ImgurDownloader, instagram::InstagramDownloader,
        mastodon::MastodonDownloader, reddit::RedditDownloader, tumblr::TumblrDownloader,
        twitter::TwitterDownloader, yt_dlp::YtDlpDownloader, Downloader,
    },
    DownloadFileRequest, DownloaderReturn,
};

#[derive(Debug)]
pub struct DownloadHandler {
    can_handle: fn(&str) -> bool,
    handler: Box<dyn Downloader>,
}
impl DownloadHandler {
    pub fn new<T>(can_handle: fn(&str) -> bool, handler: T) -> Self
    where
        T: Downloader + 'static,
    {
        Self {
            can_handle,
            handler: Box::new(handler),
        }
    }

    #[must_use]
    pub fn can_handle(&self, file: &DownloadFileRequest) -> bool {
        (self.can_handle)(&file.original_url)
    }

    #[must_use]
    pub fn download(&self, file: &DownloadFileRequest) -> DownloaderReturn {
        (self.handler).download(file)
    }

    #[must_use]
    pub fn try_download(&self, file: &DownloadFileRequest) -> Option<DownloaderReturn> {
        if self.can_handle(file) {
            Some(self.download(file))
        } else {
            None
        }
    }
}

pub static DEFAULT_DOWNLOAD_HANDLERS: Lazy<Vec<DownloadHandler>> = Lazy::new(|| {
    vec![
        DownloadHandler::new(InstagramDownloader::is_post_url, InstagramDownloader),
        DownloadHandler::new(TwitterDownloader::is_post_url, TwitterDownloader),
        DownloadHandler::new(TwitterDownloader::is_media_url, GenericDownloader),
        DownloadHandler::new(TumblrDownloader::is_post_url, TumblrDownloader),
        DownloadHandler::new(RedditDownloader::is_media_url, RedditDownloader),
        DownloadHandler::new(ImgurDownloader::is_media_url, ImgurDownloader),
        DownloadHandler::new(ImgurDownloader::is_post_url, ImgurDownloader),
        DownloadHandler::new(MastodonDownloader::is_mastodon_toot, MastodonDownloader),
        DownloadHandler::new(|_| true, YtDlpDownloader),
    ]
});

use super::{ExtractInfoRequest, ExtractedInfo, Extractor};
use crate::downloaders::handlers::yt_dlp::YtDlpDownloader;

#[must_use]
pub fn is_reddit_image_url(url: &str) -> bool {
    url.starts_with("https://i.redd.it/")
}

#[derive(Debug, Default)]
pub struct RedditExtractor;

#[async_trait::async_trait]
impl Extractor for RedditExtractor {
    fn name(&self) -> &'static str {
        "reddit"
    }

    fn description(&self) -> &'static str {
        "Gets reddit media. Only works on media links (eg. https://i.redd.it/...)"
    }

    async fn can_handle(&self, request: &ExtractInfoRequest) -> bool {
        Self::is_media_url(request.url.as_str())
    }

    async fn extract_info(&self, request: &ExtractInfoRequest) -> Result<ExtractedInfo, String> {
        Ok(ExtractedInfo::from_url(request, request.url.as_str())
            .with_preferred_downloader(Some(YtDlpDownloader)))
    }
}

impl RedditExtractor {
    #[must_use]
    pub fn is_media_url(url: &str) -> bool {
        url.starts_with("https://i.redd.it/")
    }
}

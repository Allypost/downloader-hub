use super::{ExtractInfoRequest, ExtractedInfo, Extractor};
use crate::downloaders::handlers::music::MusicDownloader;

#[derive(Debug, Default)]
pub struct MusicExtractor;

#[async_trait::async_trait]
impl Extractor for MusicExtractor {
    fn name(&self) -> &'static str {
        "music"
    }

    fn description(&self) -> &'static str {
        "Download songs from Spotify, Deezer, Tidal, and various other music providers. Depends on \
         external services so may be randomly unavailable."
    }

    async fn can_handle(&self, request: &ExtractInfoRequest) -> bool {
        MusicDownloader::supports(&request.url)
    }

    async fn extract_info(&self, request: &ExtractInfoRequest) -> Result<ExtractedInfo, String> {
        Ok(ExtractedInfo::from_url(request, request.url.as_str())
            .with_preferred_downloader(Some(MusicDownloader)))
    }
}

pub mod spotifydown;
pub mod yams;

use std::path::{Path, PathBuf};

use app_logger::warn;
use once_cell::sync::Lazy;
use url::Url;

use super::{DownloadRequest, Downloader, DownloaderReturn};
use crate::downloaders::DownloadResult;

static HANDLERS: Lazy<Vec<DownloadHandler>> = Lazy::new(|| {
    vec![
        DownloadHandler::new(yams::YamsProvider),
        DownloadHandler::new(spotifydown::SpotifydownProvider),
    ]
});

#[derive(Debug, Default)]
pub struct MusicDownloader;

#[async_trait::async_trait]
impl Downloader for MusicDownloader {
    fn name(&self) -> &'static str {
        "music"
    }

    fn description(&self) -> &'static str {
        "Download songs from Spotify, Deezer, Tidal, and various other music providers. Depends on \
         external services so may be randomly unavailable."
    }

    async fn can_download(&self, request: &DownloadRequest) -> bool {
        Self::supports(request.url.url())
    }

    async fn download(&self, req: &DownloadRequest) -> DownloaderReturn {
        let song_url = req.url.url();

        for handler in HANDLERS.iter() {
            if !handler.supports(song_url) {
                continue;
            }

            match handler.download(req.download_dir(), song_url).await {
                Ok(path) => {
                    return Ok(DownloadResult {
                        path,
                        request: req.clone(),
                    })
                }
                Err(e) => {
                    warn!(?e, "Failed to download song");
                }
            }
        }

        Err("No handler succeeded for song".to_string())
    }
}

impl MusicDownloader {
    pub fn supports(song_url: &Url) -> bool {
        HANDLERS.iter().any(|handler| handler.supports(song_url))
    }
}

#[derive(Debug)]
struct DownloadHandler {
    provider: Box<dyn Handler>,
}
impl DownloadHandler {
    fn new<T>(provider: T) -> Self
    where
        T: Handler + 'static,
    {
        Self {
            provider: Box::new(provider),
        }
    }

    #[must_use]
    pub fn supports(&self, url: &Url) -> bool {
        self.provider.supports(url)
    }

    pub async fn download(&self, download_dir: &Path, url: &Url) -> Result<PathBuf, anyhow::Error> {
        self.provider.download(download_dir, url).await
    }
}

#[async_trait::async_trait]
trait Handler: std::fmt::Debug + Send + Sync {
    async fn download(&self, download_dir: &Path, song_url: &Url) -> anyhow::Result<PathBuf>;

    fn supports(&self, song_url: &Url) -> bool;
}

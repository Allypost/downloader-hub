pub mod spotifydown;
pub mod yams;

use std::path::{Path, PathBuf};

use app_logger::warn;
use once_cell::sync::Lazy;
use url::Url;

use super::{DownloadUrlInfo, Downloader, DownloaderReturn, ResolvedDownloadFileRequest};
use crate::downloaders::DownloadResult;

static HANDLERS: Lazy<Vec<DownloadHandler>> = Lazy::new(|| {
    vec![
        DownloadHandler::new(yams::YamsProvider),
        DownloadHandler::new(spotifydown::SpotifydownProvider),
    ]
});

#[derive(Debug, Default)]
pub struct MusicDownloader;
impl Downloader for MusicDownloader {
    fn name(&self) -> &'static str {
        "music"
    }

    fn description(&self) -> &'static str {
        "Download songs from Spotify, Deezer, Tidal, and various other music providers. Depends on \
         external services so may be randomly unavailable."
    }

    fn get_resolved(
        &self,
        req: &super::DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        Ok(ResolvedDownloadFileRequest {
            resolved_urls: vec![DownloadUrlInfo::from_url(&req.original_url)],
            request_info: req.clone(),
        })
    }

    fn download_resolved(&self, resolved_file: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        for info in &resolved_file.resolved_urls {
            let Ok(song_url) = Url::parse(&info.url) else {
                continue;
            };
            for handler in HANDLERS.iter() {
                if !handler.supports(&song_url) {
                    continue;
                }

                match handler.download(&resolved_file.request_info.download_dir, &song_url) {
                    Ok(path) => {
                        return vec![Ok(DownloadResult {
                            path,
                            request: resolved_file.request_info.clone(),
                        })]
                    }
                    Err(e) => {
                        warn!(?e, "Failed to download song");
                    }
                }
            }
        }

        vec![Err("No handler found for song".to_string())]
    }
}

impl MusicDownloader {
    pub fn supports(song_url: &str) -> bool {
        let Ok(song_url) = Url::parse(song_url) else {
            return false;
        };

        HANDLERS.iter().any(|handler| handler.supports(&song_url))
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

    pub fn download(&self, download_dir: &Path, url: &Url) -> Result<PathBuf, anyhow::Error> {
        self.provider.download(download_dir, url)
    }
}

trait Handler: std::fmt::Debug + Send + Sync {
    fn download(&self, download_dir: &Path, song_url: &Url) -> anyhow::Result<PathBuf>;

    fn supports(&self, song_url: &Url) -> bool;
}

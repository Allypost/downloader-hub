use std::fmt::Debug;

pub use common::{
    download_request::{DownloadRequest, DownloaderOptions},
    download_result::DownloadResult,
};
pub use handlers::DownloaderEntry;
use serde::Serialize;

mod common;
pub mod handlers;
mod helpers;

pub use handlers::AVAILABLE_DOWNLOADERS;
use tracing::{debug, info};

#[async_trait::async_trait]
pub trait Downloader: Debug + Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn can_run(&self) -> bool {
        true
    }

    async fn can_download(&self, request: &DownloadRequest) -> bool;

    async fn download(&self, req: &DownloadRequest) -> DownloaderReturn;
}
impl std::fmt::Display for dyn Downloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Downloader::{}", self.name())
    }
}
impl Serialize for dyn Downloader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

pub type DownloaderReturn = Result<DownloadResult, DownloaderError>;
pub type DownloaderError = String;

pub async fn download_file(file: &DownloadRequest) -> DownloaderReturn {
    info!(?file, "Downloading file");

    let new_file_paths = download_file_with(&AVAILABLE_DOWNLOADERS, file).await;

    debug!("Downloaded files: {:?}", &new_file_paths);

    new_file_paths
}

pub async fn download_file_with(
    downloaders: &[DownloaderEntry],
    request: &DownloadRequest,
) -> DownloaderReturn {
    async fn find_downloader(
        downloaders: &[DownloaderEntry],
        request: &DownloadRequest,
    ) -> Option<DownloaderEntry> {
        if let Some(downloader) = &request.preferred_downloader {
            if downloader.can_download(request).await {
                return Some(downloader.clone());
            }
        }

        for downloader in downloaders {
            if downloader.can_download(request).await {
                return Some(downloader.clone());
            }
        }

        None
    }

    let downloader = find_downloader(downloaders, request).await;

    let downloader = match downloader {
        Some(d) => d,
        None => {
            return Err(format!(
                "Could not find a downloader that can handle {r:?}",
                r = request,
            ));
        }
    };

    downloader.download(request).await
}

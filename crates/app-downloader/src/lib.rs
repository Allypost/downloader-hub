use app_logger::{debug, info};
use downloaders::{DownloadFileRequest, DownloadResult, DownloaderReturn};
pub use handler::default_download_handlers;
use handler::DownloadHandler;

use crate::handler::DEFAULT_DOWNLOAD_HANDLERS;

pub mod common;
pub mod downloaders;
pub mod handler;

pub fn download_file(file: &DownloadFileRequest) -> DownloaderReturn {
    info!(?file, "Downloading file");

    let new_file_paths = download_file_with(&DEFAULT_DOWNLOAD_HANDLERS, file);

    debug!("Downloaded files: {:?}", &new_file_paths);

    new_file_paths
}

#[must_use]
pub fn download_file_with(
    downloaders: &[DownloadHandler],
    file: &DownloadFileRequest,
) -> DownloaderReturn {
    let downloader = downloaders.iter().find(|d| d.can_handle(file));
    let downloader = match downloader {
        Some(d) => d,
        None => {
            return vec![Err(format!(
                "Could not find a downloader that can handle {url}",
                url = file.original_url,
            ))];
        }
    };

    downloader.download(file)
}

pub mod generic;
pub mod music;
pub mod yt_dlp;

use std::sync::Arc;

use once_cell::sync::Lazy;

pub use super::{
    common::{download_request::DownloadRequest, download_result::DownloadResult},
    Downloader, DownloaderError, DownloaderReturn,
};

pub type DownloaderEntry = Arc<dyn Downloader>;

pub static ALL_DOWNLOADERS: Lazy<Vec<DownloaderEntry>> = Lazy::new(all_downloaders);

pub static AVAILABLE_DOWNLOADERS: Lazy<Vec<DownloaderEntry>> = Lazy::new(available_downloaders);

fn all_downloaders() -> Vec<DownloaderEntry> {
    vec![
        Arc::new(yt_dlp::YtDlp),
        Arc::new(generic::Generic),
        Arc::new(music::Music),
    ]
}

#[must_use]
fn available_downloaders() -> Vec<DownloaderEntry> {
    all_downloaders()
        .into_iter()
        .filter(|x| x.can_run())
        .collect()
}

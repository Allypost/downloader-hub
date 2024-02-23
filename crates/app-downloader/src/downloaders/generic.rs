use std::{ffi::OsString, fs::File, path::PathBuf, string::ToString};

use app_helpers::id::time_id;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use unicode_segmentation::UnicodeSegmentation;
use url::Url;

use super::{Downloader, ResolvedDownloadFileRequest};
use crate::{common::request::Client, DownloadFileRequest, DownloadResult, DownloaderReturn};

pub const MAX_FILENAME_LENGTH: usize = 120;

#[derive(Debug, Default)]
pub struct GenericDownloader;

#[async_trait::async_trait]
impl Downloader for GenericDownloader {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        Ok(ResolvedDownloadFileRequest {
            resolved_urls: vec![req.original_url.clone()],
            request_info: req.clone(),
        })
    }

    fn download_resolved(&self, resolved: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        resolved
            .resolved_urls
            .par_iter()
            .map(|url| self.download_one(&resolved.request_info, url))
            .collect::<Vec<_>>()
    }
}

impl GenericDownloader {
    pub fn download_one(
        &self,
        request_info: &DownloadFileRequest,
        url: &str,
    ) -> Result<DownloadResult, String> {
        app_logger::info!(?url, dir = ?request_info.download_dir, "Downloading with generic downloader");

        let mut res = Client::from_download_request(request_info, url)?
            .send()
            .map_err(|e| format!("Failed to send request: {:?}", e))?
            .error_for_status()
            .map_err(|e| format!("Failed to get response: {:?}", e))?;

        let mime_type = res.headers().get("content-type").map(|x| x.to_str());
        app_logger::debug!(?mime_type, "Got mime type");
        let mime_type = match mime_type {
            Some(Ok(mime_type)) => mime_type,
            _ => "",
        };

        let extension =
            mime2ext::mime2ext(mime_type).map_or("unknown".to_string(), |x| (*x).to_string());

        app_logger::debug!(?extension, "Got extension");

        let id = time_id();
        let mut file_name = OsString::from(&id);

        let taken_filename_len = id.len() + 1 + extension.len();

        let url_file_name = Url::parse(url)
            .ok()
            .map(|x| PathBuf::from(x.path()))
            .and_then(|x| {
                let stem = x.file_stem()?;

                let trunc = stem
                    .to_string_lossy()
                    .graphemes(true)
                    .filter(|x| !x.chars().all(char::is_control))
                    .filter(|x| !x.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']))
                    .take(MAX_FILENAME_LENGTH - 1 - taken_filename_len)
                    .collect::<String>();

                if trunc.is_empty() {
                    None
                } else {
                    Some(trunc)
                }
            });

        if let Some(url_file_name) = url_file_name {
            app_logger::trace!(?url_file_name, "Got url file name");
            file_name.push(".");
            file_name.push(url_file_name);
        }

        file_name.push(".");
        file_name.push(extension);

        let file_path = request_info.download_dir.join(file_name);
        app_logger::debug!(?file_path, "Writing to file");
        let mut out_file =
            File::create(&file_path).map_err(|e| format!("Failed to create file: {:?}", e))?;

        res.copy_to(&mut out_file)
            .map_err(|e| format!("Failed to copy response to file: {:?}", e))?;

        Ok(DownloadResult {
            request: request_info.clone(),
            path: file_path,
        })
    }
}

#[must_use]
pub fn download(request: &DownloadFileRequest) -> DownloaderReturn {
    GenericDownloader.download(request)
}

pub fn download_one(request: &DownloadFileRequest, url: &str) -> Result<DownloadResult, String> {
    GenericDownloader.download_one(request, url)
}

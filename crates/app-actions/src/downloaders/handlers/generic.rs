use std::{ffi::OsString, path::PathBuf, string::ToString};

use app_helpers::id::time_id;
use http::header;
use mime2ext::mime2ext;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{debug, info, trace};
use unicode_segmentation::UnicodeSegmentation;
use url::Url;

use super::{DownloadRequest, DownloadResult, Downloader, DownloaderReturn};
use crate::{common::request::Client, downloaders::helpers::headers::content_disposition};

pub const MAX_FILENAME_LENGTH: usize = 120;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Generic;

#[async_trait::async_trait]
#[typetag::serde]
impl Downloader for Generic {
    fn description(&self) -> &'static str {
        "Just tries to download exactly what you give it. No fancy tricks."
    }

    async fn can_download(&self, req: &DownloadRequest) -> bool {
        matches!(req.url.url().scheme(), "http" | "https")
    }

    async fn download(&self, request: &DownloadRequest) -> DownloaderReturn {
        self.download_one(request).await
    }
}

impl Generic {
    pub async fn download_one(
        &self,
        request_info: &DownloadRequest,
    ) -> Result<DownloadResult, String> {
        let url = &request_info.url;

        info!(?url, dir = ?request_info.download_dir(), "Downloading with generic downloader");

        let mut res = Client::base_with_url(url)?
            .headers(url.headers().clone())
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {:?}", e))?
            .error_for_status()
            .map_err(|e| format!("Failed to get response: {:?}", e))?;

        let mime_type = res.headers().get(header::CONTENT_TYPE).map(|x| x.to_str());
        debug!(?mime_type, "Got mime type");
        let mime_type = match mime_type {
            Some(Ok(mime_type)) => mime_type,
            _ => "",
        };

        let extension = mime2ext(mime_type).map_or("unknown".to_string(), |x| (*x).to_string());

        debug!(?extension, "Got extension");

        let id = time_id();
        let mut file_name = OsString::from(&id);

        let taken_filename_len = id.len() + 1 + extension.len();

        let req_file_name = res
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|x| content_disposition::ContentDisposition::from_raw(x).ok())
            .and_then(|x| {
                debug!(?x, "Got content disposition");
                x.get_filename_ext()
                    .and_then(content_disposition::ExtendedValue::try_decode)
                    .or_else(|| x.get_filename().map(ToString::to_string))
            })
            .or_else(|| {
                let url = url.url();
                debug!(?url, "Using url as filename");
                url_to_filename(url, taken_filename_len)
            });

        trace!(?req_file_name, "Got file name from request");

        if let Some(url_file_name) = req_file_name {
            file_name.push(".");
            file_name.push(url_file_name);
        }

        file_name.push(".");
        file_name.push(extension);

        let file_path = request_info.download_dir().join(file_name);
        debug!(?file_path, "Writing to file");
        let mut out_file = File::create(&file_path)
            .await
            .map_err(|e| format!("Failed to create file: {:?}", e))?;

        while let Some(chunk) = res
            .chunk()
            .await
            .map_err(|e| format!("Failed to get chunk: {:?}", e))?
        {
            out_file
                .write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {:?}", e))?;
        }

        Ok(DownloadResult {
            request: request_info.clone(),
            path: file_path,
        })
    }
}

fn url_to_filename(url: &Url, taken_filename_len: usize) -> Option<String> {
    Some(url).map(|x| PathBuf::from(x.path())).and_then(|x| {
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
    })
}

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::download_request::DownloadRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub request: DownloadRequest,
    pub path: PathBuf,
}

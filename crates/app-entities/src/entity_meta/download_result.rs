use serde::{Deserialize, Serialize};

use super::common::path::AppPath;
use crate::{download_result, sea_orm_active_enums::ItemStatus};

impl download_result::Model {
    #[must_use]
    pub fn path(&self) -> Option<AppPath> {
        self.path.clone().and_then(|x| AppPath::try_from(x).ok())
    }

    #[must_use]
    pub fn meta(&self) -> Option<DownloadResultMeta> {
        serde_json::from_value(self.meta.clone()).ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadResultStatus {
    Failed(String),
    Pending,
    Processing,
    Success,
}
impl DownloadResultStatus {
    #[must_use]
    pub fn as_item_status(&self) -> ItemStatus {
        self.clone().into()
    }
}

impl From<DownloadResultStatus> for ItemStatus {
    fn from(status: DownloadResultStatus) -> Self {
        match status {
            DownloadResultStatus::Failed(_) => Self::Failed,
            DownloadResultStatus::Pending => Self::Pending,
            DownloadResultStatus::Processing => Self::Processing,
            DownloadResultStatus::Success => Self::Success,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadResultMeta {
    Error(String),
    FileData(DownloadResultMetaFileData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResultMetaFileData {
    pub hash: String,
    pub size: Option<i64>,
    pub file_type: Option<String>,
}

impl From<DownloadResultMeta> for serde_json::Value {
    fn from(meta: DownloadResultMeta) -> Self {
        serde_json::to_value(meta).expect("Invalid download result meta")
    }
}

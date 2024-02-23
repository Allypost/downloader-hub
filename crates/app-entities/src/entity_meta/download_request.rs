use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::download_request;

impl download_request::Model {
    #[must_use]
    pub fn app_meta(&self) -> Option<DownloadRequestAppMeta> {
        self.app_meta.clone().try_into().ok()
    }

    #[must_use]
    pub fn meta(&self) -> Option<DownloadRequestMeta> {
        serde_json::from_value(self.meta.clone()).ok()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequestMetaRequest {
    #[serde(with = "http_serde::method", default = "default_get")]
    pub method: http::Method,
    #[serde(with = "http_serde::header_map", default)]
    pub headers: http::HeaderMap,
}
const fn default_get() -> http::Method {
    http::Method::GET
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequestMeta {
    #[serde(default)]
    pub request: DownloadRequestMetaRequest,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub skip_fixing: bool,
    #[serde(default)]
    pub other: HashMap<String, serde_json::Value>,
}
impl From<DownloadRequestMeta> for serde_json::Value {
    fn from(meta: DownloadRequestMeta) -> Self {
        serde_json::to_value(meta).expect("Invalid download request meta")
    }
}

// pub type DownloadRequestMeta = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadRequestAppMeta {
    Error(String),
    Info(DownloadRequestAppMetaInfo),
}

impl From<DownloadRequestAppMeta> for serde_json::Value {
    fn from(value: DownloadRequestAppMeta) -> Self {
        serde_json::to_value(value).expect("Invalid app meta value")
    }
}

impl TryFrom<serde_json::Value> for DownloadRequestAppMeta {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequestAppMetaInfo {
    pub request_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequestWithHidden {
    pub id: i32,
    pub client_id: i32,
    #[serde(flatten)]
    pub request: download_request::Model,
    pub app_meta: serde_json::Value,
}
impl From<download_request::Model> for DownloadRequestWithHidden {
    fn from(request: download_request::Model) -> Self {
        Self {
            id: request.id,
            client_id: request.client_id,
            app_meta: request.app_meta.clone(),
            request,
        }
    }
}

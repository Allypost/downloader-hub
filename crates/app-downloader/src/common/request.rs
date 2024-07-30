use std::time::Duration;

use reqwest::blocking::{
    Client as ReqwestClient, ClientBuilder as ReqwestClientBuilder,
    RequestBuilder as ReqwestRequestBuilder,
};

use super::USER_AGENT;
use crate::downloaders::{DownloadFileRequest, DownloadUrlInfo};

pub struct Client;

impl Client {
    pub fn base() -> Result<ReqwestClient, String> {
        Self::builder()
            .build()
            .map_err(|e| format!("Failed to create client: {:?}", e))
    }

    pub fn from_download_request_and_url(
        req: &DownloadFileRequest,
        url: &DownloadUrlInfo,
    ) -> Result<ReqwestRequestBuilder, String> {
        let mut builder = Self::base()?
            .request(
                req.method.as_str().parse().expect("Failed to parse method"),
                url.url(),
            )
            .timeout(Duration::from_secs(5));

        for (k, v) in &req.headers {
            builder = builder.header(k, v);
        }

        Ok(builder)
    }

    pub fn from_download_request(
        req: &DownloadFileRequest,
        url: &str,
    ) -> Result<ReqwestRequestBuilder, String> {
        Self::from_download_request_and_url(req, &DownloadUrlInfo::from_url(url))
    }

    pub fn builder() -> ReqwestClientBuilder {
        ReqwestClient::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(5))
    }
}

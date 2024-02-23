use std::time::Duration;

use reqwest::blocking::{
    Client as ReqwestClient, ClientBuilder as ReqwestClientBuilder,
    RequestBuilder as ReqwestRequestBuilder,
};

use super::USER_AGENT;
use crate::downloaders::DownloadFileRequest;

pub struct Client;

impl Client {
    pub fn base() -> Result<ReqwestClient, String> {
        Self::builder()
            .build()
            .map_err(|e| format!("Failed to create client: {:?}", e))
    }

    pub fn from_download_request(
        req: &DownloadFileRequest,
        url: &str,
    ) -> Result<ReqwestRequestBuilder, String> {
        let builder = Self::base()?
            .request(
                req.method.as_str().parse().expect("Failed to parse method"),
                url,
            )
            // .headers(req.headers)
            .timeout(Duration::from_secs(5));

        Ok(builder)
    }

    pub fn builder() -> ReqwestClientBuilder {
        ReqwestClient::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(5))
    }
}

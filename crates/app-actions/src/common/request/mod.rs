use std::time::Duration;

pub use reqwest::{Client as RequestClient, ClientBuilder as RequestClientBuilder, RequestBuilder};

use super::url::UrlWithMeta;

pub const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like \
                              Gecko) Chrome/88.0.4324.182 Safari/537.36";

const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct Client;

impl Client {
    pub fn base() -> Result<RequestClient, String> {
        Self::builder()
            .build()
            .map_err(|e| format!("Failed to create client: {:?}", e))
    }

    pub fn base_with_url(url: &UrlWithMeta) -> Result<RequestBuilder, String> {
        let mut builder = Self::base()?.request(url.method().clone(), url.url().as_str());

        for (k, v) in url.headers() {
            builder = builder.header(k, v);
        }

        Ok(builder)
    }

    pub fn builder() -> RequestClientBuilder {
        RequestClient::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
    }
}

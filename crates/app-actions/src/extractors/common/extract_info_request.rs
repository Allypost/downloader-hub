use http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::common::request::{Client, RequestBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractInfoRequest {
    pub url: Url,
    #[serde(with = "http_serde::method", default = "default_get")]
    pub method: Method,
    #[serde(with = "http_serde::header_map", default)]
    pub headers: HeaderMap,
}

impl ExtractInfoRequest {
    #[must_use]
    pub fn new<T>(url: T) -> Self
    where
        T: Into<Url>,
    {
        Self {
            url: url.into(),
            method: Method::GET,
            headers: HeaderMap::default(),
        }
    }

    pub fn as_request_builder(&self) -> Result<RequestBuilder, String> {
        let mut builder = Client::base()?.request(
            self.method
                .as_str()
                .parse()
                .expect("Failed to parse method"),
            self.url.as_str(),
        );

        for (k, v) in &self.headers {
            builder = builder.header(k, v);
        }

        Ok(builder)
    }
}

const fn default_get() -> Method {
    Method::GET
}

impl From<Url> for ExtractInfoRequest {
    fn from(url: Url) -> Self {
        Self::new(url)
    }
}

impl From<&Url> for ExtractInfoRequest {
    fn from(url: &Url) -> Self {
        url.clone().into()
    }
}

impl TryFrom<&str> for ExtractInfoRequest {
    type Error = url::ParseError;

    fn try_from(url: &str) -> Result<Self, Self::Error> {
        let parsed_url = Url::parse(url)?;

        Ok(parsed_url.into())
    }
}

impl TryFrom<String> for ExtractInfoRequest {
    type Error = url::ParseError;

    fn try_from(url: String) -> Result<Self, Self::Error> {
        url.as_str().try_into()
    }
}

impl TryFrom<&String> for ExtractInfoRequest {
    type Error = url::ParseError;

    fn try_from(url: &String) -> Result<Self, Self::Error> {
        url.as_str().try_into()
    }
}

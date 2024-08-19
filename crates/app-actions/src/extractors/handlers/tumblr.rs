use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{twitter::Twitter, ExtractInfoRequest, ExtractedInfo, Extractor};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Tumblr;

#[async_trait::async_trait]
#[typetag::serde]
impl Extractor for Tumblr {
    fn description(&self) -> &'static str {
        "Downloads images and videos from Tumblr and screenshots the post itself."
    }

    async fn can_handle(&self, request: &ExtractInfoRequest) -> bool {
        Self::is_post_url(&request.url)
    }

    async fn extract_info(&self, request: &ExtractInfoRequest) -> Result<ExtractedInfo, String> {
        Twitter.extract_info(request).await
    }
}

static DOMAIN_MATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:(?P<subdomain>[^\-][a-zA-Z0-9\-]{0,30}[^\-])\.)?tumblr\.com")
        .expect("Invalid regex")
});

impl Tumblr {
    pub fn is_post_url(url: &Url) -> bool {
        let Some(domain) = url.domain() else {
            return false;
        };

        DOMAIN_MATCH.is_match(domain)
    }
}

use super::{ExtractInfoRequest, ExtractedInfo, Extractor};

pub struct FallthroughExtractor;

#[async_trait::async_trait]
impl Extractor for FallthroughExtractor {
    fn name(&self) -> &'static str {
        "fallthrough"
    }

    fn description(&self) -> &'static str {
        "Fallthrough extractor. Does nothing, just forwards the URL."
    }

    async fn can_handle(&self, _request: &ExtractInfoRequest) -> bool {
        true
    }

    async fn extract_info(&self, request: &ExtractInfoRequest) -> Result<ExtractedInfo, String> {
        Ok(ExtractedInfo::from_url(request, request.url.as_str()))
    }
}

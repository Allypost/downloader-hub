pub use common::{
    extract_info_request::ExtractInfoRequest,
    extracted_info::{ExtractedInfo, ExtractedUrlInfo},
};
pub use handlers::AVAILABLE_EXTRACTORS;

mod common;
pub mod handlers;

#[async_trait::async_trait]
#[typetag::serde(tag = "$extractor")]
pub trait Extractor {
    fn name(&self) -> &'static str {
        self.typetag_name()
    }

    fn description(&self) -> &'static str;

    async fn can_handle(&self, request: &ExtractInfoRequest) -> bool;

    async fn extract_info(&self, request: &ExtractInfoRequest) -> Result<ExtractedInfo, String>;
}

pub async fn extract_info(request: &ExtractInfoRequest) -> Result<ExtractedInfo, String> {
    for extractor in AVAILABLE_EXTRACTORS.iter() {
        if extractor.can_handle(request).await {
            return extractor.extract_info(request).await.map(|x| {
                x.with_meta(
                    "extractor",
                    serde_json::to_value(extractor).expect("Failed to serialize extractor"),
                )
            });
        }
    }

    Err("No extractor found".to_string())
}

pub mod activity_pub;
pub mod bsky;
pub mod fallthough;
pub mod imgur;
pub mod instagram;
pub mod music;
pub mod reddit;
pub mod tiktok;
pub mod tumblr;
pub mod twitter;

use std::sync::Arc;

use once_cell::sync::Lazy;

use super::{ExtractInfoRequest, ExtractedInfo, Extractor};

pub type ExtractorEntry = Arc<dyn Extractor + Sync + Send>;

pub static AVAILABLE_EXTRACTORS: Lazy<Vec<ExtractorEntry>> = Lazy::new(available_extractors);

#[must_use]
pub fn available_extractors() -> Vec<ExtractorEntry> {
    vec![
        Arc::new(imgur::Imgur),
        Arc::new(instagram::Instagram),
        Arc::new(reddit::Reddit),
        Arc::new(tiktok::Tiktok),
        Arc::new(tumblr::Tumblr),
        Arc::new(twitter::Twitter),
        Arc::new(music::Music),
        Arc::new(bsky::Bsky),
        Arc::new(activity_pub::ActivityPub),
        Arc::new(fallthough::Fallthrough),
    ]
}

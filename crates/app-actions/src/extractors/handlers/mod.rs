pub mod activity_pub;
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
        Arc::new(imgur::ImgurExtractor),
        Arc::new(instagram::InstagramExtractor),
        Arc::new(reddit::RedditExtractor),
        Arc::new(tiktok::TiktokExtractor),
        Arc::new(tumblr::TumblrExtractor),
        Arc::new(twitter::TwitterExtractor),
        Arc::new(music::MusicExtractor),
        Arc::new(activity_pub::ActivityPubExtractor),
        Arc::new(fallthough::FallthroughExtractor),
    ]
}

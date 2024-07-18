use std::{path::Path, string::ToString};

use app_config::Config;
use app_logger::{debug, trace};
use http::{header, HeaderMap};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use url::{form_urlencoded, Url};

use super::{
    generic::GenericDownloader, yt_dlp::YtDlpDownloader, DownloadFileRequest, DownloadResult,
    Downloader, DownloaderError, ResolvedDownloadFileRequest,
};
use crate::{common::request::Client, DownloaderReturn};

pub static URL_MATCH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^https?://(www\.)?(twitter|x)\.com/(?P<username>[^/]+)/status/(?P<status_id>[0-9]+)",
    )
    .expect("Invalid regex")
});

pub static MEDIA_URL_MATCH: Lazy<Regex> = Lazy::new(|| {
    // https://pbs.twimg.com/media/FqPFEWYWYBQ5iG3?format=png&name=small
    Regex::new(r"^https?://pbs\.twimg\.com/media/").expect("Invalid regex")
});

static DEFAULT_AUTHORIZATION: &str =
    "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%\
     3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

static TWEET_INFO_ENDPOINT: &str =
    "https://x.com/i/api/graphql/0hWvDhmW8YQ-S_ib3azIrw/TweetResultByRestId";

static GUEST_TOKEN_ENDPOINT: &str = "https://api.twitter.com/1.1/guest/activate.json";

#[derive(Debug, Default)]
pub struct TwitterDownloader;

#[async_trait::async_trait]
impl Downloader for TwitterDownloader {
    fn name(&self) -> &'static str {
        "twitter"
    }

    #[app_logger::instrument]
    fn get_resolved(
        &self,
        req: &DownloadFileRequest,
    ) -> Result<ResolvedDownloadFileRequest, String> {
        debug!("Downloading tweet");

        let tweet_info = get_tweet_info_from_url(&req.original_url)?;

        trace!(?tweet_info, "Got tweet info");

        let tweet_data = get_tweet_data(&tweet_info.status_id)?;

        trace!(?tweet_data, "Got tweet data");

        let mut tweet_media = get_tweet_media_urls(&tweet_data).unwrap_or_default();

        trace!(?tweet_media, "Got tweet media");

        let tweet_screenshot_url = {
            let endpoint = &Config::global().endpoint.twitter_screenshot_base_url;

            format!("{}/{}", endpoint.trim_end_matches('/'), req.original_url)
        };

        trace!("Adding Tweet screenshot URL: {:?}", &tweet_screenshot_url);
        tweet_media.push(TweetMedia::Photo {
            url: tweet_screenshot_url,
        });

        let resolved_urls = tweet_media
            .into_iter()
            .map(|x| x.as_url().to_string())
            .collect();

        Ok(ResolvedDownloadFileRequest {
            request_info: req.clone(),
            resolved_urls,
        })
    }

    fn download_resolved(&self, resolved: &ResolvedDownloadFileRequest) -> DownloaderReturn {
        YtDlpDownloader.download_resolved(resolved)
    }
}

impl TwitterDownloader {
    pub fn download_media_url(
        &self,
        download_dir: &Path,
        twitter_media_url: &str,
    ) -> Result<DownloadResult, String> {
        let mut parsed = Url::parse(twitter_media_url)
            .map_err(|x| format!("Failed to parse twitter media URL: {x:?}"))?;

        let url_without_name = {
            let params = parsed.query_pairs().filter(|(key, _)| key != "name");
            let params = form_urlencoded::Serializer::new(String::new())
                .clear()
                .extend_pairs(params)
                .finish();

            parsed.set_query(Some(&params));

            parsed.to_string()
        };

        GenericDownloader.download_one(
            &DownloadFileRequest::new(twitter_media_url, download_dir),
            &url_without_name,
        )
    }

    pub fn screenshot_tweet(
        &self,
        download_dir: &Path,
        url: &str,
    ) -> Result<DownloadResult, DownloaderError> {
        debug!(?url, "Trying to screenshot tweet");

        let endpoint = &Config::global().endpoint.twitter_screenshot_base_url;
        let tweet_screenshot_url = format!("{}/{}", endpoint.trim_end_matches('/'), url);

        trace!(url = ?tweet_screenshot_url, "Tweet screenshot URL");

        GenericDownloader.download_one(
            &DownloadFileRequest::new(url, download_dir),
            &tweet_screenshot_url,
        )
    }

    pub fn is_post_url(url: &str) -> bool {
        URL_MATCH.is_match(url)
    }

    pub fn is_media_url(url: &str) -> bool {
        MEDIA_URL_MATCH.is_match(url)
    }
}

pub fn download(req: &DownloadFileRequest) -> DownloaderReturn {
    debug!(?req, "Trying to download tweet media");

    let yt_dlp_result = YtDlpDownloader.download(req);

    if let Some(Err(_)) = yt_dlp_result.first() {
        debug!("Failed to download with yt-dlp. Trying to screenshot...");
        vec![screenshot_tweet(&req.download_dir, &req.original_url)]
    } else {
        yt_dlp_result
    }
}

pub fn download_media_url(
    download_dir: &Path,
    twitter_media_url: &str,
) -> Result<DownloadResult, String> {
    TwitterDownloader.download_media_url(download_dir, twitter_media_url)
}

fn screenshot_tweet(download_dir: &Path, url: &str) -> Result<DownloadResult, DownloaderError> {
    TwitterDownloader.screenshot_tweet(download_dir, url)
}

#[derive(Debug)]
#[allow(dead_code)]
struct TweetInfo {
    username: String,
    status_id: String,
}
#[app_logger::instrument]
fn get_tweet_info_from_url(url: &str) -> Result<TweetInfo, String> {
    let caps = URL_MATCH.captures(url).ok_or("Invalid tweet URL")?;

    trace!(?caps, "Tweet URL match");

    let username = caps
        .name("username")
        .ok_or_else(|| "Couldn't get username from URL match".to_string())?
        .as_str();

    let status_id = caps
        .name("status_id")
        .ok_or_else(|| "Couldn't get status id from URL match".to_string())?
        .as_str();

    Ok(TweetInfo {
        username: username.to_string(),
        status_id: status_id.to_string(),
    })
}

#[derive(Debug)]
struct TweetData(serde_json::Value);

#[app_logger::instrument]
fn get_tweet_data(tweet_id: &str) -> Result<TweetData, String> {
    let guest_auth = get_guest_auth()?;

    let query_params = {
        let graphql_variables = json!({
            "tweetId": tweet_id,
            "referrer": "home",
            "with_rux_injections": false,
            "includePromotedContent": false,
            "withCommunity": false,
            "withQuickPromoteEligibilityTweetFields": false,
            "withBirdwatchNotes": false,
            "withVoice": false,
            "withV2Timeline": false,
        })
        .to_string();
        let graphql_features = json!({
            "rweb_lists_timeline_redesign_enabled": true,
            "responsive_web_graphql_exclude_directive_enabled": true,
            "verified_phone_label_enabled": true,
            "creator_subscriptions_tweet_preview_api_enabled": true,
            "responsive_web_graphql_timeline_navigation_enabled": true,
            "responsive_web_graphql_skip_user_profile_image_extensions_enabled": false,
            "tweetypie_unmention_optimization_enabled": true,
            "responsive_web_edit_tweet_api_enabled": true,
            "graphql_is_translatable_rweb_tweet_is_translatable_enabled": true,
            "view_counts_everywhere_api_enabled": true,
            "longform_notetweets_consumption_enabled": true,
            "responsive_web_twitter_article_tweet_consumption_enabled": false,
            "tweet_awards_web_tipping_enabled": false,
            "freedom_of_speech_not_reach_fetch_enabled": true,
            "standardized_nudges_misinfo": true,
            "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled": true,
            "longform_notetweets_rich_text_read_enabled": true,
            "longform_notetweets_inline_media_enabled": true,
            "responsive_web_media_download_video_enabled": false,
            "responsive_web_enhance_cards_enabled": false,
        })
        .to_string();

        &[
            ("variables", graphql_variables),
            ("features", graphql_features),
        ]
    };

    trace!(?query_params, "Tweet info query params");

    let url = {
        let mut url = Url::parse(TWEET_INFO_ENDPOINT).expect("Invalid URL");
        url.query_pairs_mut().extend_pairs(query_params.iter());

        url.to_string()
    };

    trace!(?url, "Tweet info URL");

    let resp = Client::base()?
        .get(url)
        .headers(guest_auth.get_headers())
        .send()
        .map_err(|e| format!("Failed to send request: {:?}", e))?
        .json::<serde_json::Value>()
        .map_err(|e| format!("Failed to parse response: {:?}", e))?;

    trace!(?resp, "Got response");

    resp.as_object()
        .and_then(|x| x.get("data"))
        .and_then(|x| x.as_object())
        .and_then(|x| x.get("tweetResult"))
        .and_then(|x| x.as_object())
        .and_then(|x| x.get("result"))
        .map(|x| TweetData(x.clone()))
        .ok_or_else(|| {
            format!(
                "Failed to get tweet data from response: {:?}",
                resp.to_string()
            )
        })
}

#[derive(Debug)]
pub enum TweetMedia {
    Photo { url: String },
    Video { url: String },
}
impl TweetMedia {
    #[must_use]
    pub fn as_url(&self) -> &str {
        match self {
            Self::Photo { url } | Self::Video { url } => url,
        }
    }
}

#[app_logger::instrument(skip(tweet_data))]
fn get_tweet_media_urls(tweet_data: &TweetData) -> Option<Vec<TweetMedia>> {
    #[derive(Debug, Clone, Deserialize)]
    #[allow(dead_code)]
    struct RespTweetMediaVideoVariant {
        bitrate: Option<i64>,
        content_type: String,
        url: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct RespTweetMediaVideo {
        variants: Vec<RespTweetMediaVideoVariant>,
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(tag = "type")]
    enum RespTweetMedia {
        #[serde(rename = "photo")]
        Photo {
            #[serde(rename = "media_url_https")]
            url: String,
        },
        #[serde(rename = "video")]
        #[serde(alias = "animated_gif")]
        Video { video_info: RespTweetMediaVideo },
    }
    impl RespTweetMedia {
        fn into_media(self) -> Option<TweetMedia> {
            match self {
                Self::Photo { url } => Some(TweetMedia::Photo { url }),
                Self::Video { video_info } => {
                    let mut variants = video_info.variants;
                    variants.sort_by(|lt, gt| {
                        gt.bitrate
                            .unwrap_or_default()
                            .cmp(&lt.bitrate.unwrap_or_default())
                    });

                    variants
                        .first()
                        .map(|x| TweetMedia::Video { url: x.url.clone() })
                }
            }
        }
    }

    trace!(?tweet_data, "Getting tweet media from tweet data");

    let mut tweet_media = tweet_data
        .0
        .get("legacy")
        .and_then(|x| x.as_object())
        .and_then(|x| x.get("extended_entities"))
        .and_then(|x| x.as_object())
        .and_then(|x| x.get("media"))
        .and_then(|x| serde_json::from_value::<Vec<RespTweetMedia>>(x.clone()).ok())
        .map(|x| {
            x.into_iter()
                .filter_map(RespTweetMedia::into_media)
                .collect::<Vec<_>>()
        });

    trace!(?tweet_media, "Got tweet media");

    if let Some(tm) = &tweet_media {
        if tm.is_empty() {
            trace!("No tweet media found");
            tweet_media = None;
        }
    }

    tweet_media
}

#[derive(Debug)]
struct GuestAuth {
    guest_token: String,
    cookie: Option<String>,
}
impl GuestAuth {
    fn get_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            DEFAULT_AUTHORIZATION
                .try_into()
                .expect("Invalid header value"),
        );
        headers.insert(
            "x-guest-token",
            self.guest_token
                .as_str()
                .try_into()
                .expect("Invalid header value"),
        );

        if let Some(cookie) = &self.cookie {
            headers.insert(
                header::COOKIE,
                cookie.try_into().expect("Invalid header value"),
            );
        }

        headers
    }
}

#[app_logger::instrument]
fn get_guest_auth() -> Result<GuestAuth, String> {
    #[derive(Deserialize)]
    struct GetGuestIdResponse {
        guest_token: String,
    }

    debug!("Getting Twitter guest auth token");

    let resp = Client::base()?
        .post(GUEST_TOKEN_ENDPOINT)
        .header(header::AUTHORIZATION, DEFAULT_AUTHORIZATION)
        .send()
        .map_err(|e| format!("Failed to send request: {:?}", e))?;

    trace!(?resp, "Got response");

    let cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|x| x.to_str().ok())
        .and_then(|x| x.split(';').next())
        .map(ToString::to_string);

    let guest_token = resp
        .json::<GetGuestIdResponse>()
        .map_err(|e| format!("Failed to parse response: {:?}", e))
        .map(|x| x.guest_token)?;

    debug!(?guest_token, ?cookie, "Got guest auth token");

    Ok(GuestAuth {
        guest_token,
        cookie,
    })
}

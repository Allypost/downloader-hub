use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use clap::{Args, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};
use url::Url;
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[allow(clippy::struct_field_names)]
#[clap(next_help_heading = Some("Program paths"))]
pub struct ProgramPathConfig {
    /// Path to the yt-dlp executable.
    ///
    /// If not provided, yt-dlp will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_YT_DLP", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom = "valid_path", required)]
    yt_dlp_path: Option<PathBuf>,

    /// Path to the ffmpeg executable.
    ///
    /// If not provided, ffmpeg will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_FFMPEG", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom = "valid_path", required)]
    ffmpeg_path: Option<PathBuf>,

    /// Path to the ffprobe executable.
    ///
    /// If not provided, ffprobe will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_FFPROBE", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom = "valid_path", required)]
    ffprobe_path: Option<PathBuf>,

    /// Path to the scenedetect executable.
    ///
    /// If not provided, scenedetect will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_SCENEDETECT", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom = "valid_path")]
    scenedetect_path: Option<PathBuf>,
}
impl ProgramPathConfig {
    pub fn yt_dlp_path(&self) -> &PathBuf {
        self.yt_dlp_path.as_ref().expect(
            "`yt-dlp` executable not found. Please make sure it is installed and added to the \
             PATH environment variable.",
        )
    }

    pub fn ffmpeg_path(&self) -> &PathBuf {
        self.ffmpeg_path.as_ref().expect(
            "`ffmpeg` executable not found. Please make sure it is installed and added to the \
             PATH environment variable.",
        )
    }

    pub fn ffprobe_path(&self) -> &PathBuf {
        self.ffprobe_path.as_ref().expect(
            "`ffprobe` executable not found. Please make sure it is installed and added to the \
             PATH environment variable.",
        )
    }

    pub fn scenedetect_path(&self) -> Option<PathBuf> {
        self.scenedetect_path.clone()
    }

    pub(crate) fn resolve_paths(mut self) -> Self {
        self.yt_dlp_path = self.yt_dlp_path.or_else(|| which::which("yt-dlp").ok());
        self.ffmpeg_path = self.ffmpeg_path.or_else(|| which::which("ffmpeg").ok());
        self.ffprobe_path = self.ffprobe_path.or_else(|| which::which("ffprobe").ok());
        self.scenedetect_path = self
            .scenedetect_path
            .or_else(|| which::which("scenedetect").ok());

        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = Some("External endpoints/APIs"))]
pub struct EndpointConfig {
    /// The base URL for the Twitter screenshot API.
    #[arg(long, default_value = "https://twitter.igr.ec", env = "DOWNLOADER_HUB_ENDPOINT_TWITTER_SCREENSHOT", value_hint = ValueHint::Url, value_parser = validate_absolute_url())]
    #[validate(custom = "absolute_url")]
    pub twitter_screenshot_base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum DumpConfigType {
    Json,
    Toml,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[allow(clippy::option_option)]
#[clap(next_help_heading = Some("Run options"))]
pub struct RunConfig {
    /// Dump the config to stdout
    #[arg(long, value_enum, default_value = None)]
    pub dump_config: Option<Option<DumpConfigType>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = Some("Server options"))]
pub struct ServerConfig {
    /// The port on which the server will listen.
    #[arg(long, default_value = "8000", env = "PORT", value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,

    /// The host on which the server will listen.
    #[arg(long, default_value = "127.0.0.1", env = "HOST")]
    pub host: String,

    /// The admin key for the server.
    /// Used to authenticate admin requests.
    /// Should be at least 32 characters long and securely random.
    #[arg(long, env = "DOWNLOADER_HUB_ADMIN_KEY", value_parser = validate_min_key_length(32))]
    #[validate(length(min = 32))]
    pub admin_key: String,

    /// The key used for signing various tokens.
    /// Should be at least 32 characters long and securely random.
    #[arg(long, env = "DOWNLOADER_HUB_SIGNING_KEY", value_parser = validate_min_key_length(32))]
    #[validate(length(min = 32))]
    pub signing_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = "Database options")]
pub struct DatabaseConfig {
    /// PostgreSQL database URL.
    ///
    /// Should be in the format of `postgres://username:password@db-host:5432/database-name`
    #[clap(long = "database-url", env = "DATABASE_URL")]
    #[validate(url)]
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = "Application options")]
pub struct AppConfig {
    /// The public URL where the application is served.
    /// This is used to generate links to the application.
    /// Should be in the format of `https://www.example.com/some/path` or `http://127.0.0.1:8000`
    #[clap(long, env = "DOWNLOADER_HUB_PUBLIC_URL", value_hint = ValueHint::Url, value_parser = validate_absolute_url())]
    #[validate(custom = "absolute_url")]
    pub public_url: String,
}

pub fn validate_min_key_length(min_len: usize) -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        if s.len() < min_len {
            return Err(format!(
                "Key must be at least 32 characters long. Currently it's {} characters long.",
                s.len()
            ));
        }

        Ok(s.to_string())
    }
}

pub fn validate_valid_path() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        let path = Path::new(s);
        if !path.exists() {
            return Err("File does not exist");
        }

        Ok(path.to_path_buf())
    }
}

pub fn valid_path(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() {
        return Err(ValidationError::new("File does not exist"));
    }

    if !path.is_file() {
        return Err(ValidationError::new("Path is not a valid file"));
    }

    Ok(())
}

pub fn validate_absolute_url() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        let parsed = match Url::parse(s) {
            Ok(parsed) => parsed,
            Err(e) => return Err(format!("URL must be absolute: {e}")),
        };

        if parsed.cannot_be_a_base() {
            return Err("URL must be absolute".to_string());
        }

        Ok(s.trim_end_matches('/').to_string())
    }
}

pub fn absolute_url<'a, T>(url: T) -> Result<(), ValidationError>
where
    T: Into<Cow<'a, str>>,
{
    let parsed =
        Url::parse(url.into().as_ref()).map_err(|_| ValidationError::new("Invalid URL"))?;

    if parsed.cannot_be_a_base() {
        return Err(ValidationError::new("URL must be absolute"));
    }

    Ok(())
}

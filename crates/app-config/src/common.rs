use std::path::{Path, PathBuf};

use clap::{Args, CommandFactory, ValueEnum, ValueHint};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use url::Url;
use validator::{Validate, ValidationError};

use crate::{
    cli::CliArgs,
    timeframe::Timeframe,
    validators::{
        file::{validate_is_file, value_parser_parse_valid_file},
        url::{
            validate_is_absolute_url, value_parser_parse_absolute_url,
            value_parser_parse_absolute_url_as_url,
        },
    },
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[allow(clippy::struct_field_names)]
#[clap(next_help_heading = Some("Program paths"))]
pub struct ProgramPathConfig {
    /// Path to the yt-dlp executable.
    ///
    /// If not provided, yt-dlp will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_YT_DLP", value_hint = ValueHint::FilePath, value_parser = value_parser_parse_valid_file())]
    #[validate(custom(function = "validate_is_file"), required)]
    yt_dlp_path: Option<PathBuf>,

    /// Path to the ffmpeg executable.
    ///
    /// If not provided, ffmpeg will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_FFMPEG", value_hint = ValueHint::FilePath, value_parser = value_parser_parse_valid_file())]
    #[validate(custom(function = "validate_is_file"), required)]
    ffmpeg_path: Option<PathBuf>,

    /// Path to the ffprobe executable.
    ///
    /// If not provided, ffprobe will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_FFPROBE", value_hint = ValueHint::FilePath, value_parser = value_parser_parse_valid_file())]
    #[validate(custom(function = "validate_is_file"), required)]
    ffprobe_path: Option<PathBuf>,

    /// Path to the scenedetect executable.
    ///
    /// If not provided, scenedetect will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_SCENEDETECT", value_hint = ValueHint::FilePath, value_parser = value_parser_parse_valid_file())]
    #[validate(custom(function = "validate_is_file"))]
    scenedetect_path: Option<PathBuf>,

    /// Path to the imagemagick executable.
    ///
    /// If not provided, imagemagick will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_IMAGEMAGICK", value_hint = ValueHint::FilePath, value_parser = value_parser_parse_valid_file())]
    #[validate(custom(function = "validate_is_file"))]
    imagemagick_path: Option<PathBuf>,
}
impl ProgramPathConfig {
    #[must_use]
    pub fn yt_dlp_path(&self) -> &Path {
        self.yt_dlp_path.as_ref().expect(
            "`yt-dlp` executable not found. Please make sure it is installed and added to the \
             PATH environment variable.",
        )
    }

    #[must_use]
    pub fn ffmpeg_path(&self) -> &Path {
        self.ffmpeg_path.as_ref().expect(
            "`ffmpeg` executable not found. Please make sure it is installed and added to the \
             PATH environment variable.",
        )
    }

    #[must_use]
    pub fn ffprobe_path(&self) -> &Path {
        self.ffprobe_path.as_ref().expect(
            "`ffprobe` executable not found. Please make sure it is installed and added to the \
             PATH environment variable.",
        )
    }

    #[must_use]
    pub fn scenedetect_path(&self) -> Option<PathBuf> {
        self.scenedetect_path.clone()
    }

    #[must_use]
    pub fn imagemagick_path(&self) -> Option<PathBuf> {
        self.imagemagick_path.clone()
    }

    #[must_use]
    pub fn resolve_paths(mut self) -> Self {
        self.with_resolved_paths();
        self
    }

    pub fn with_resolved_paths(&mut self) -> &Self {
        self.yt_dlp_path = self
            .yt_dlp_path
            .clone()
            .or_else(|| which::which("yt-dlp").ok());
        self.ffmpeg_path = self
            .ffmpeg_path
            .clone()
            .or_else(|| which::which("ffmpeg").ok());
        self.ffprobe_path = self
            .ffprobe_path
            .clone()
            .or_else(|| which::which("ffprobe").ok());

        self.scenedetect_path = self
            .scenedetect_path
            .clone()
            .or_else(|| which::which("scenedetect").ok());

        self.imagemagick_path = self
            .imagemagick_path
            .clone()
            .or_else(|| which::which("magick").ok());

        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = Some("External endpoints/APIs"))]
pub struct EndpointConfig {
    /// The base URL for the Twitter screenshot API.
    #[arg(long, default_value = "https://twitter.igr.ec", env = "DOWNLOADER_HUB_ENDPOINT_TWITTER_SCREENSHOT", value_hint = ValueHint::Url, value_parser = value_parser_parse_absolute_url())]
    #[validate(custom(function = "validate_is_absolute_url"))]
    pub twitter_screenshot_base_url: String,

    /// The base URL for the OCR API.
    #[arg(long, env = "DOWNLOADER_HUB_ENDPOINT_OCR_API", value_hint = ValueHint::Url, value_parser = value_parser_parse_absolute_url_as_url())]
    pub ocr_api_base_url: Option<Url>,
}
impl EndpointConfig {
    #[must_use]
    pub fn ocr_api_url(&self, path: &str) -> Option<Url> {
        self.ocr_api_base_url
            .as_ref()
            .and_then(|x| x.join(path.trim_start_matches('/')).ok())
    }
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

    /// Dump shell completions to stdout
    #[arg(long, default_value = None, value_name = "SHELL", value_parser = hacky_dump_completions())]
    #[serde(skip)]
    pub dump_completions: Option<Shell>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = Some("Task options"))]
pub struct TaskConfig {
    /// The interval at which the bot should check for updates to the yt-dlp binary.
    /// If not set, the bot will not check for updates.
    ///
    /// The value represents a duration in seconds, minutes, hours, days, weeks, or months.
    /// Special events are ignored, eg. leap years, daylight savings, etc.
    /// `minute` is 60 seconds, `hour` is 60 minutes, `day` is 24 hours, `week` is 7 days, `month` is 30 days.
    /// Eg. 1d, 2 weeks, 3 months, 4h, 5mins, 6s
    #[clap(short, long, value_parser = Timeframe::parse_str, env = "DOWNLOADER_HUB_YT_DLP_UPDATE_INTERVAL")]
    pub yt_dlp_update_interval: Option<Timeframe>,
}

#[must_use]
pub fn hacky_dump_completions() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        let parsed = Shell::from_str(s, true);

        if let Ok(shell) = &parsed {
            let bin_name = std::env::current_exe()
                .map_err(|_e| ValidationError::new("Unknown application name"))?
                .file_name()
                .map(|x| x.to_string_lossy().to_string())
                .ok_or_else(|| ValidationError::new("Unknown application name"))?;

            clap_complete::generate(
                *shell,
                &mut CliArgs::command(),
                bin_name,
                &mut std::io::stdout(),
            );
            std::process::exit(0);
        }

        parsed
            .map(|_| ())
            .map_err(|_| ValidationError::new("Invalid shell"))
    }
}

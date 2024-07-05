use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use clap::{Args, CommandFactory, ValueEnum, ValueHint};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use url::Url;
use validator::{Validate, ValidationError};

use crate::cli::CliArgs;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[allow(clippy::struct_field_names)]
#[clap(next_help_heading = Some("Program paths"))]
pub struct ProgramPathConfig {
    /// Path to the yt-dlp executable.
    ///
    /// If not provided, yt-dlp will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_YT_DLP", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom(function = "valid_path"), required)]
    yt_dlp_path: Option<PathBuf>,

    /// Path to the ffmpeg executable.
    ///
    /// If not provided, ffmpeg will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_FFMPEG", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom(function = "valid_path"), required)]
    ffmpeg_path: Option<PathBuf>,

    /// Path to the ffprobe executable.
    ///
    /// If not provided, ffprobe will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_FFPROBE", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom(function = "valid_path"), required)]
    ffprobe_path: Option<PathBuf>,

    /// Path to the scenedetect executable.
    ///
    /// If not provided, scenedetect will be searched for in $PATH
    #[arg(long, default_value = None, env = "DOWNLOADER_HUB_SCENEDETECT", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom(function = "valid_path"))]
    scenedetect_path: Option<PathBuf>,
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

        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = Some("External endpoints/APIs"))]
pub struct EndpointConfig {
    /// The base URL for the Twitter screenshot API.
    #[arg(long, default_value = "https://twitter.igr.ec", env = "DOWNLOADER_HUB_ENDPOINT_TWITTER_SCREENSHOT", value_hint = ValueHint::Url, value_parser = validate_absolute_url())]
    #[validate(custom(function = "absolute_url"))]
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

    /// Dump shell completions to stdout
    #[arg(long, default_value = None, value_name = "SHELL", value_parser = hacky_dump_completions())]
    #[serde(skip)]
    pub dump_completions: Option<Shell>,
}

#[must_use]
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

#[must_use]
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

#[must_use]
pub fn hacky_dump_completions() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        let parsed = Shell::from_str(s, true);

        if let Ok(shell) = &parsed {
            let bin_name = if cfg!(feature = "server") {
                "downloader-hub"
            } else if cfg!(feature = "cli") {
                "downloader-cli"
            } else {
                return Err(ValidationError::new("Unknown application name"));
            };

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

#[must_use]
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

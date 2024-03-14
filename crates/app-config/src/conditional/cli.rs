use std::path::PathBuf;

use clap::Args;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = "Cli options")]
pub struct CliConfig {
    #[clap(flatten)]
    #[validate]
    #[serde(skip)]
    pub entries_group: UrlGroup,

    /// Directory to download files to
    ///
    /// Will be created if it doesn't exist.
    ///
    /// Will error if it is not a valid path.
    #[clap(short, long, default_value = ".")]
    pub download_directory: PathBuf,
}

#[derive(Debug, Clone, Default, Args, Serialize, Deserialize)]
#[group(required = true, multiple = true)]
pub struct UrlGroup {
    /// URLs to download.
    ///
    /// Has the same behaviour as specifying the entry as a raw argument.
    /// Will be checked whether they are valid urls or not.
    ///
    /// Errors will be thrown if any urls are invalid.
    #[clap(short = 'u', long = "url")]
    pub urls: Vec<String>,

    /// Paths to fix.
    ///
    /// Paths will be resolved and checked whether they are valid paths or not.
    ///
    /// Errors will be thrown if any paths are invalid or if they don't exist.
    #[clap(short = 'f', long = "file")]
    pub files: Vec<PathBuf>,

    /// Download entry to process
    ///
    /// Entry can be either an url or a path.
    /// Multiple entries can be specified.
    ///
    /// If a path is specified, the file at the path will be run through fixers, and urls will be downloaded.
    ///
    /// Invalid entries will be _ignored_.
    #[clap(id = "URL_OR_FILE")]
    pub urls_or_files: Vec<DownloadEntry>,
}

pub type DownloadEntry = String;

use std::path::{Path, PathBuf};

use clap::{Args, ValueHint};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use crate::common::validate_valid_path;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Args, Validate)]
#[clap(next_help_heading = "Cli options")]
pub struct CliConfig {
    #[clap(flatten)]
    #[validate(nested)]
    #[serde(skip)]
    pub entries_group: UrlGroup,

    /// Directory to download files to
    ///
    /// Will be created if it doesn't exist.
    ///
    /// Will error if it is not a valid path.
    #[clap(short = 'd', long, default_value = ".", value_hint = ValueHint::FilePath, value_parser = validate_valid_resolved_directory())]
    #[validate(custom(function = "valid_directory"))]
    pub output_directory: PathBuf,
}

#[derive(Debug, Clone, Default, Args, Serialize, Deserialize, Validate)]
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
    #[clap(short = 'f', long = "file", value_hint = ValueHint::FilePath, value_parser = validate_valid_path())]
    #[validate(custom(function = "valid_files"))]
    pub files: Vec<PathBuf>,

    /// Download entry to process
    ///
    /// Entry can be either an url or a path.
    /// Multiple entries can be specified.
    ///
    /// If a path is specified, the file at the path will be run through fixers, and urls will be downloaded.
    ///
    /// Invalid entries will be _ignored_.
    #[clap(id = "URL_OR_FILE", value_hint = ValueHint::FilePath)]
    pub urls_or_files: Vec<DownloadEntry>,
}

pub type DownloadEntry = String;

#[must_use]
pub fn validate_valid_resolved_directory() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        let path = Path::new(s);

        if !path.exists() {
            return Err("File does not exist");
        }

        if !path.is_dir() {
            return Err("Path is not a directory");
        }

        let path = path
            .to_path_buf()
            .canonicalize()
            .map_err(|_| "Failed to canonicalize path")?;

        Ok(path)
    }
}

pub fn valid_files(paths: &Vec<PathBuf>) -> Result<(), ValidationError> {
    for path in paths {
        if !path.exists() {
            return Err(ValidationError::new("File does not exist"));
        }

        if !path.is_file() {
            return Err(ValidationError::new("Path is not a valid file"));
        }
    }

    Ok(())
}

pub fn valid_directory(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() {
        return Err(ValidationError::new("Directory does not exist"));
    }

    if !path.is_dir() {
        return Err(ValidationError::new("Path is not a directory"));
    }

    Ok(())
}

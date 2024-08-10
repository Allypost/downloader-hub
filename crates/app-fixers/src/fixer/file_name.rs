use std::{
    fs,
    path::{Path, PathBuf},
};

use app_logger::{debug, trace};
use thiserror::Error;

use crate::{error::FixerError, Fixer, FixerOptions, FixerReturn, IntoFixerReturn};

#[derive(Debug)]
pub struct FileName;
#[async_trait::async_trait]
impl Fixer for FileName {
    fn name(&self) -> &'static str {
        "file-name"
    }

    fn description(&self) -> &'static str {
        "Fix file name to contain only approved characters."
    }

    /// Options:
    ///
    fn run(&self, file_path: &Path, _options: &FixerOptions) -> FixerReturn {
        fix_file_name(file_path)
    }
}

pub fn fix_file_name(file_path: &Path) -> FixerReturn {
    debug!("Checking file name for {file_path:?}...");
    let name = file_path.file_stem().and_then(|x| return x.to_str());

    let new_name = match name {
        Some(name) if !name.is_ascii() => {
            debug!("File name {name:?} contains non-ascii characters. Trying to fix...");
            name.replace(|c: char| !c.is_ascii(), "")
        }
        None => {
            return FileNameError::NoName(file_path.to_path_buf()).into_fixer_return();
        }
        Some(name) => {
            debug!("File name for {name:?} is OK. Skipping...");
            return Ok(vec![file_path.to_path_buf()]);
        }
    };

    let extension = file_path
        .extension()
        .and_then(|x| return x.to_str())
        .ok_or_else(|| FileNameError::NoExtension(file_path.to_path_buf()))
        .map_err(FixerError::failed_fix)?;

    trace!("New file name: {new_name:?} (extension: {extension:?}) for file {file_path:?}");

    let new_name = format!("{new_name}.{extension}");
    let new_file_path = file_path.with_file_name(new_name);

    debug!("Renaming file from {file_path:?} to {new_file_path:?}");

    fs::rename(file_path, &new_file_path)
        .map(|()| vec![new_file_path])
        .map_err(FileNameError::Rename)
        .map_err(FixerError::failed_fix)
}

#[derive(Debug, Error)]
pub enum FileNameError {
    #[error("Failed to get name for file {0:?}")]
    NoName(PathBuf),
    #[error("Failed to get extension for file {0:?}")]
    NoExtension(PathBuf),
    #[error("Failed to rename file: {0:?}")]
    Rename(std::io::Error),
}

impl IntoFixerReturn for FileNameError {
    fn into_fixer_return(self) -> FixerReturn {
        Err(crate::error::FixerError::failed_fix(self))
    }
}

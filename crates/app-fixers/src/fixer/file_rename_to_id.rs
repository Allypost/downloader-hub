use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use app_helpers::id::time_id;
use thiserror::Error;

use crate::{error::FixerError, Fixer, FixerOptions, FixerReturn};

#[derive(Debug)]
pub struct RenameToId;
#[async_trait::async_trait]
impl Fixer for RenameToId {
    fn name(&self) -> &'static str {
        "rename-to-id"
    }

    fn description(&self) -> &'static str {
        "Rename file to match standard file naming convention \
         ($TIME_ID.$ORIGINAL_CROPPED_NAME.$EXT)"
    }

    /// Options:
    ///
    fn run(&self, file_path: &Path, _options: &FixerOptions) -> FixerReturn {
        rename_file_to_id(file_path)
    }
}

pub fn rename_file_to_id(file_path: &Path) -> FixerReturn {
    let original_file_name = match file_path.file_stem().and_then(OsStr::to_str) {
        Some(file_name) => file_name,
        None => return Err(RenameToIdError::NoFileName(file_path.to_path_buf()).into()),
    };
    let original_file_ext = match file_path.extension().and_then(OsStr::to_str) {
        Some(file_ext) => file_ext,
        None => return Err(RenameToIdError::NoFileExtension(file_path.to_path_buf()).into()),
    };

    let id = time_id();
    let new_file_name = {
        let new_name = format!(
            "{id}.{name}.{ext}",
            id = id,
            name = original_file_name,
            ext = original_file_ext
        );

        file_path.with_file_name(new_name)
    };

    std::fs::rename(file_path, &new_file_name).map_err(RenameToIdError::Rename)?;

    Ok(vec![new_file_name])
}

#[derive(Debug, Error)]
pub enum RenameToIdError {
    #[error("Couldn't get file name for {0:?}")]
    NoFileName(PathBuf),
    #[error("Couldn't get file extension for {0:?}")]
    NoFileExtension(PathBuf),
    #[error("Failed to rename file: {0:?}")]
    Rename(std::io::Error),
}

impl From<RenameToIdError> for FixerError {
    fn from(value: RenameToIdError) -> Self {
        Self::FailedFix(value.into())
    }
}

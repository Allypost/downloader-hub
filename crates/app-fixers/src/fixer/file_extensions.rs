use std::{ffi::OsStr, fs, path::PathBuf};

use app_logger::{debug, trace};
use thiserror::Error;

use crate::{FixerReturn, IntoFixerReturn};

pub fn fix_file_extension(file_path: &PathBuf) -> FixerReturn {
    debug!("Checking file extension for {file_path:?}...");

    let extension = file_path.extension().and_then(OsStr::to_str);

    let file_ext = match infer::get_from_path(file_path) {
        Ok(Some(ext)) => ext.extension(),
        _ => {
            return FileExtensionError::UnableToGetExtension(file_path.clone()).into_fixer_return();
        }
    };
    debug!("Inferred file extension: {:?}", file_ext);

    if let Some(extension) = extension {
        if extension == file_ext {
            debug!("File extension is correct");
            return Ok(file_path.clone());
        }
    }

    trace!(
        "File extension is incorrect ({:?} vs ({:?}))",
        extension,
        file_ext
    );

    let new_file_path = file_path.with_extension(file_ext);

    debug!("Renaming file from {file_path:?} to {new_file_path:?}");
    match fs::rename(file_path, &new_file_path) {
        Ok(()) => Ok(new_file_path),
        Err(e) => FileExtensionError::UnableToRenameFile(e).into_fixer_return(),
    }
}

#[derive(Debug, Error)]
pub enum FileExtensionError {
    #[error("Unable to get extension of {0:?}")]
    UnableToGetExtension(PathBuf),
    #[error("Unable to rename file: {0:?}")]
    UnableToRenameFile(std::io::Error),
}

impl IntoFixerReturn for FileExtensionError {
    fn into_fixer_return(self) -> FixerReturn {
        Err(crate::error::FixerError::FailedFix(self.into()))
    }
}

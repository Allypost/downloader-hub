use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use app_logger::{debug, trace};
use thiserror::Error;

use crate::{Fixer, FixerOptions, FixerReturn, IntoFixerReturn};

#[derive(Debug)]
pub struct FileExtension;
#[async_trait::async_trait]
impl Fixer for FileExtension {
    fn name(&self) -> &'static str {
        "file-extension"
    }

    fn description(&self) -> &'static str {
        "Fix file extensions to match the file type."
    }

    /// Options:
    ///
    fn run(&self, file_path: &Path, _options: &FixerOptions) -> FixerReturn {
        fix_file_extension(file_path)
    }
}

pub fn fix_file_extension(file_path: &Path) -> FixerReturn {
    debug!("Checking file extension for {file_path:?}...");

    let extension = file_path.extension().and_then(OsStr::to_str);

    let file_ext = match infer::get_from_path(file_path) {
        Ok(Some(ext)) => ext.extension(),
        _ => {
            return FileExtensionError::UnableToGetExtension(file_path.to_path_buf())
                .into_fixer_return();
        }
    };
    debug!("Inferred file extension: {:?}", file_ext);

    if let Some(extension) = extension {
        if extension == file_ext {
            debug!("File extension is correct");
            return Ok(vec![file_path.to_path_buf()]);
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
        Ok(()) => Ok(vec![new_file_path]),
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

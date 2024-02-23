extern crate scopeguard;

use std::path::{Path, PathBuf};

use error::FixerError;

mod common;
pub mod error;
pub mod fixer;
mod util;

pub static DEFAULT_FIXERS: &[Fixer] = &[
    fixer::file_extensions::fix_file_extension,
    fixer::file_name::fix_file_name,
    fixer::media_formats::convert_into_preferred_formats,
    fixer::crop::auto_crop_video,
];

pub fn fix_file(path: &Path) -> Result<PathBuf, FixerError> {
    sync::fix_file_with(DEFAULT_FIXERS, path)
}

pub mod as_future {
    use std::path::{Path, PathBuf};

    use resolve_path::PathResolveExt;

    use crate::{error::FixerError, Fixer, IntoFixerReturn, DEFAULT_FIXERS};

    pub async fn fix_files_with(
        fixers: &[Fixer],
        paths: &[PathBuf],
    ) -> Result<Vec<PathBuf>, FixerError> {
        let res = paths
            .iter()
            .map(|path| fix_file_with(fixers, path))
            .collect::<Vec<_>>();

        futures::future::join_all(res)
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
    }

    pub async fn fix_file(path: &Path) -> Result<PathBuf, FixerError> {
        fix_file_with(DEFAULT_FIXERS, path).await
    }

    pub async fn fix_file_with(fixers: &[Fixer], path: &Path) -> Result<PathBuf, FixerError> {
        let p = path
            .resolve()
            .canonicalize()
            .map_err(|e| FixerError::FailedToCanonicalizePath(path.to_path_buf(), e))?;

        let fixers = fixers.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut p = p.clone();
            for filter in fixers {
                p = filter(&p).into_fixer_return()?;
            }

            Ok(p)
        })
        .await
        .map_err(FixerError::JoinError)?
    }
}

pub mod sync {
    use std::path::{Path, PathBuf};

    use app_helpers::futures::run_async;

    use crate::{error::FixerError, Fixer, DEFAULT_FIXERS};

    pub fn fix_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, FixerError> {
        fix_files_with(DEFAULT_FIXERS, paths)
    }

    pub fn fix_files_with(fixers: &[Fixer], paths: &[PathBuf]) -> Result<Vec<PathBuf>, FixerError> {
        run_async(crate::as_future::fix_files_with(fixers, paths))
    }

    pub fn fix_file(path: &Path) -> Result<PathBuf, FixerError> {
        fix_file_with(DEFAULT_FIXERS, path)
    }

    pub fn fix_file_with(fixers: &[Fixer], path: &Path) -> Result<PathBuf, FixerError> {
        run_async(crate::as_future::fix_file_with(fixers, path))
    }
}

type Fixer = fn(&PathBuf) -> FixerReturn;

type FixerReturn = Result<PathBuf, FixerError>;
trait IntoFixerReturn {
    fn into_fixer_return(self) -> FixerReturn;
}
impl IntoFixerReturn for FixerReturn {
    fn into_fixer_return(self) -> FixerReturn {
        self
    }
}

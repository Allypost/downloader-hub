use std::{
    collections::HashMap,
    convert::Into,
    path::{Path, PathBuf},
};

use error::FixerError;
pub use fixer::default_fixers;
use fixer::FixerInstance;
use resolve_path::PathResolveExt;
use util::transferable_file_times;

mod common;
pub mod error;
pub mod fixer;
mod util;

pub async fn fix_file(path: &Path) -> FixerReturn {
    fix_file_with(fixer::default_fixers(), path).await
}

pub async fn fix_file_with(fixers: Vec<FixerInstance>, path: &Path) -> FixerReturn {
    let p = path
        .try_resolve()
        .map_err(|e| FixerError::FailedToResolvePath(path.to_path_buf(), e))?;
    let p = p
        .canonicalize()
        .map_err(|e| FixerError::FailedToCanonicalizePath(p.to_path_buf(), e))?;

    if !p.exists() {
        return Err(FixerError::FileNotFound(p));
    }

    if !p.is_file() {
        return Err(FixerError::NotAFile(p));
    }

    let transfer_file_times = transferable_file_times(&p);

    let res = tokio::task::spawn_blocking(move || {
        let mut ps = vec![p];
        for fixer in fixers {
            let new_p = ps
                .into_iter()
                .map(|p| fixer.run(&p, &HashMap::new()))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();

            ps = new_p;
        }

        Ok::<_, FixerError>(ps)
    })
    .await
    .map_err(FixerError::JoinError)??;

    if let Ok(transfer_file_times) = transfer_file_times {
        for new_path in &res {
            if new_path.as_os_str() != path.as_os_str() {
                if let Err(e) = transfer_file_times(new_path) {
                    app_logger::warn!(
                        "Failed to transfer file times of {path:?} to {new_path:?}: {e:?}"
                    );
                }
            }
        }
    }

    Ok(res)
}

pub type FixerReturn = Result<Vec<PathBuf>, FixerError>;
pub type FixerOptions = HashMap<String, String>;
#[async_trait::async_trait]
pub trait Fixer: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    fn run(&self, file_path: &Path, options: &FixerOptions) -> FixerReturn;

    #[allow(unused_variables)]
    fn can_run(&self, file_path: &Path, options: &FixerOptions) -> bool {
        true
    }
}

pub trait IntoFixerReturn {
    fn into_fixer_return(self) -> FixerReturn;
}
impl<T, E> IntoFixerReturn for Result<T, E>
where
    T: Into<Vec<PathBuf>>,
    E: Into<FixerError>,
{
    fn into_fixer_return(self) -> FixerReturn {
        self.map(Into::into).map_err(Into::into)
    }
}

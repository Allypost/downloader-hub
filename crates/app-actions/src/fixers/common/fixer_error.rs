use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FixerError {
    #[error(transparent)]
    CommandError(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    FailedFix(Box<dyn std::error::Error + Send + Sync>),
    #[error("Failed to resolve path {0:?}: {1:?}")]
    FailedToResolvePath(PathBuf, #[source] std::io::Error),
    #[error("Failed to canonicalize path {0:?}: {1:?}")]
    FailedToCanonicalizePath(PathBuf, #[source] std::io::Error),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
    #[error("File {0:?} not found")]
    FileNotFound(PathBuf),
    #[error("{0:?} is not a file")]
    NotAFile(PathBuf),
}
impl FixerError {
    pub fn failed_fix<T>(err: T) -> Self
    where
        T: std::error::Error + Send + Sync + 'static,
    {
        Self::FailedFix(err.into())
    }
}

mod download_request;

use download_request::DownloadRequestHandler;

use crate::queue::task::Task;

pub static HANDLERS: &[&dyn Handler] = &[&DownloadRequestHandler];

#[async_trait::async_trait]
pub trait Handler: Sync + Send + std::fmt::Debug {
    fn name(&self) -> &'static str;

    fn can_handle(&self, task: &Task) -> bool;

    async fn handle(&self, task: &Task) -> Result<HandlerReturn, HandlerError>;
}

#[derive(Debug)]
pub struct HandlerReturn {
    pub cleanup_status_message: bool,
}
impl HandlerReturn {
    pub const fn cleanup_status_message(mut self, cleanup: bool) -> Self {
        self.cleanup_status_message = cleanup;
        self
    }
}
impl Default for HandlerReturn {
    fn default() -> Self {
        Self {
            cleanup_status_message: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Join error: `{0}`")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("IO error: `{0}`")]
    Io(#[from] tokio::io::Error),
    #[error("Fatal error: `{0}`")]
    Fatal(String),
    #[error("Failed to fix: `{0}`")]
    FixFailed(#[from] app_actions::fixers::FixerError),
}
impl HandlerError {
    pub const fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }
}

mod action_request;
mod download_request;
mod fix_request;

use crate::queue::task::Task;

pub static HANDLERS: &[&dyn Handler] = &[
    &download_request::DownloadRequestHandler,
    &fix_request::FixRequestHandler,
    &action_request::ActionRequestHandler,
];

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
    #[error("Failed to run action: `{0}`")]
    ActionFailed(#[from] app_actions::actions::ActionError),
}
impl HandlerError {
    pub const fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }

    pub const fn should_send_as_response(&self) -> bool {
        match self {
            Self::ActionFailed(x) if x.should_send_as_response() => true,
            Self::FixFailed(x) if x.should_send_as_response() => true,
            _ => false,
        }
    }
}

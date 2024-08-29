use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionError {
    #[error(transparent)]
    FailedAction(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}

impl ActionError {
    #[must_use]
    pub const fn should_send_as_response(&self) -> bool {
        matches!(self, Self::FailedAction(_))
    }
}

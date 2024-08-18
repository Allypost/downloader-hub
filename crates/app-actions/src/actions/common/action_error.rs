use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionError {
    #[error(transparent)]
    FailedAction(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}

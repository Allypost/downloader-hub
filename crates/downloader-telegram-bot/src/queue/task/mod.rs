use teloxide::types::Message;
use tracing::warn;

use crate::bot::helpers::status_message::StatusMessage;

#[derive(Clone, Debug)]
pub enum TaskInfo {
    DownloadRequest { message: Message },
}

#[derive(Clone, Debug)]
pub struct Task {
    id: String,
    info: TaskInfo,
    status_message: StatusMessage,
    retries: u32,
    added: chrono::DateTime<chrono::Utc>,
    last_run: Option<chrono::DateTime<chrono::Utc>>,
}
impl Task {
    pub fn new(info: TaskInfo, status_message: StatusMessage) -> Self {
        let mut id = ulid::Ulid::new().to_string();
        id.make_ascii_lowercase();

        Self {
            id,
            info,
            status_message,
            retries: 0,
            added: chrono::Utc::now(),
            last_run: None,
        }
    }

    pub const fn id(&self) -> &String {
        &self.id
    }

    pub const fn info(&self) -> &TaskInfo {
        &self.info
    }

    pub fn status_message(&self) -> StatusMessage {
        self.status_message.clone()
    }

    pub async fn update_status_message(&self, text: &str) {
        try_and_warn(
            self.status_message().update_message(text),
            "Failed to update status message",
        )
        .await;
    }

    pub async fn send_additional_status_message(&self, text: &str) {
        try_and_warn(
            self.status_message().send_additional_message(text),
            "Failed to send new status message",
        )
        .await;
    }

    pub fn download_request(message: Message, status_message: StatusMessage) -> Self {
        Self::new(TaskInfo::DownloadRequest { message }, status_message)
    }

    pub fn with_inc_retries(mut self) -> Self {
        self.retries += 1;
        self.last_run = Some(chrono::Utc::now());
        self
    }

    pub fn retried(&self) -> Self {
        self.clone().with_inc_retries()
    }

    pub const fn retries(&self) -> u32 {
        self.retries
    }

    pub fn time_since_added(&self) -> chrono::Duration {
        chrono::Utc::now().signed_duration_since(self.added)
    }
}

async fn try_and_warn<F, T, E, S>(f: F, error_msg: S) -> Option<T>
where
    F: std::future::Future<Output = Result<T, E>> + Send,
    E: std::fmt::Debug,
    S: Into<String> + Send,
{
    match f.await {
        Ok(res) => Some(res),
        Err(e) => {
            warn!(?e, "{}", error_msg.into());
            None
        }
    }
}

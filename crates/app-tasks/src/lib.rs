use app_logger::error;

pub(crate) mod cron;

pub struct TaskRunner;

impl TaskRunner {
    pub async fn run() -> Self {
        if let Err(e) = tokio::task::spawn_blocking(cron::spawn).await {
            error!("Failed to spawn cron tasks: {e:?}");
        }

        Self
    }
}

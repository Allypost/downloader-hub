use app_logger::error;

pub(crate) mod cron;

pub async fn start() {
    if let Err(e) = tokio::task::spawn_blocking(cron::spawn).await {
        error!("Failed to spawn cron tasks: {e:?}");
    }
}

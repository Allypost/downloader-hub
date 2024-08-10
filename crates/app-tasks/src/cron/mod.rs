use app_config::Config;
use app_logger::{debug, error, info};
use tracing::{info_span, Instrument, Span};

pub mod tasks;

#[tracing::instrument(name = "cron", skip_all)]
pub fn spawn() {
    info!("Spawning cron tasks");
    let config = Config::global();
    let task_config = &config.task;

    let span = info_span!("tasks");
    let _span = span.enter();
    if let Some(yt_dlp_update_interval) = task_config.yt_dlp_update_interval {
        debug!(interval = ?yt_dlp_update_interval, "Spawning yt-dlp update task");
        tokio::task::spawn(
            async move {
                loop {
                    tokio::time::sleep(yt_dlp_update_interval.into()).await;

                    if let Err(e) = tasks::yt_dlp::update_yt_dlp().await {
                        error!("Failed to update yt-dlp: {e:?}");
                    }
                }
            }
            .instrument(Span::current()),
        );
    }
}

use deadqueue::unlimited::Queue;
use once_cell::sync::Lazy;

pub mod processor;
pub mod task;

use crate::{
    db::AppDb,
    queue::task::Task,
    service::{download_request::DownloadRequestService, download_result::DownloadResultService},
};

pub static TASK_QUEUE: Lazy<Queue<Task>> = Lazy::new(Queue::new);

pub struct TaskQueue;
impl TaskQueue {
    pub async fn init() -> Result<(), anyhow::Error> {
        add_download_requests_from_db().await?;
        add_download_results_from_db().await?;

        Ok(())
    }
}

async fn add_download_requests_from_db() -> Result<(), anyhow::Error> {
    app_logger::info!("Checking for pending download requests");

    let db = AppDb::db();

    let pending_requests = DownloadRequestService::find_pending(&db).await?;

    app_logger::trace!(?pending_requests, "Found pending download requests");

    app_logger::debug!(
        count = pending_requests.len(),
        "Found pending download requests"
    );

    for request in &pending_requests {
        app_logger::trace!(?request, "Enqueued pending download request");
        TASK_QUEUE.push(Task::download_request(request.request_uid.clone()));
    }

    let pending_count = pending_requests.len();

    if pending_count > 0 {
        app_logger::info!(count = pending_count, "Enqueued pending download requests");
    } else {
        app_logger::info!("No pending download requests");
    }

    Ok(())
}

async fn add_download_results_from_db() -> Result<(), anyhow::Error> {
    app_logger::info!("Checking for pending download results");

    let db = AppDb::db();

    let pending_results = DownloadResultService::find_pending_results(&db).await?;

    app_logger::trace!(?pending_results, "Found pending download results");

    app_logger::debug!(
        count = pending_results.len(),
        "Found pending download results"
    );

    for result in &pending_results {
        app_logger::trace!(?result, "Enqueued pending download result");
        if let Some(path) = result.path() {
            TASK_QUEUE.push(Task::process_download_result(result.id, path));
        }
    }

    let pending_count = pending_results.len();

    if pending_count > 0 {
        app_logger::info!(count = pending_count, "Enqueued pending download results");
    } else {
        app_logger::info!("No pending download results");
    }

    Ok(())
}

use deadqueue::unlimited::Queue;
use once_cell::sync::Lazy;
use tracing::{debug, info, trace};

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
    info!("Checking for pending download requests");

    let db = AppDb::db();

    let pending_requests = DownloadRequestService::find_pending(&db).await?;

    trace!(?pending_requests, "Found pending download requests");

    debug!(
        count = pending_requests.len(),
        "Found pending download requests"
    );

    for request in &pending_requests {
        trace!(?request, "Enqueued pending download request");
        TASK_QUEUE.push(Task::download_request(request.request_uid.clone()));
    }

    let pending_count = pending_requests.len();

    if pending_count > 0 {
        info!(count = pending_count, "Enqueued pending download requests");
    } else {
        info!("No pending download requests");
    }

    Ok(())
}

async fn add_download_results_from_db() -> Result<(), anyhow::Error> {
    info!("Checking for pending download results");

    let db = AppDb::db();

    let pending_results = DownloadResultService::find_pending_results(&db).await?;

    trace!(?pending_results, "Found pending download results");

    debug!(
        count = pending_results.len(),
        "Found pending download results"
    );

    for result in &pending_results {
        trace!(?result, "Enqueued pending download result");
        if let Some(path) = result.path() {
            TASK_QUEUE.push(Task::process_download_result(result.id, path));
        }
    }

    let pending_count = pending_results.len();

    if pending_count > 0 {
        info!(count = pending_count, "Enqueued pending download results");
    } else {
        info!("No pending download results");
    }

    Ok(())
}

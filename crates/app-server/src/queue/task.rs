use app_entities::entity_meta::common::path::AppPath;

#[derive(Clone, Debug)]
pub enum TaskInfo {
    DownloadRequest(String),
    ProcessDownloadResult((i32, AppPath)),
}

#[derive(Clone, Debug)]
pub struct Task {
    info: TaskInfo,
    retries: u32,
    added: chrono::DateTime<chrono::Utc>,
    last_run: Option<chrono::DateTime<chrono::Utc>>,
}
impl Task {
    pub fn new(info: TaskInfo) -> Self {
        Self {
            info,
            retries: 0,
            added: chrono::Utc::now(),
            last_run: None,
        }
    }

    pub const fn info(&self) -> &TaskInfo {
        &self.info
    }

    pub fn download_request(request_uid: String) -> Self {
        Self::new(TaskInfo::DownloadRequest(request_uid))
    }

    pub fn process_download_result(request_id: i32, path: AppPath) -> Self {
        Self::new(TaskInfo::ProcessDownloadResult((request_id, path)))
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

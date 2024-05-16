use ulid::Ulid;

pub enum AppUidFor {
    Client,
    DownloadRequest,
    DownloadResult,
}
impl AppUidFor {
    pub fn generate(&self) -> String {
        let ulid = Ulid::new();
        let time_id = app_helpers::id::time_thread_id();

        format!("{}_{}_{}", self, ulid, time_id)
    }

    pub fn client() -> String {
        Self::Client.generate()
    }

    pub fn download_request() -> String {
        Self::DownloadRequest.generate()
    }

    pub fn download_result() -> String {
        Self::DownloadResult.generate()
    }
}
impl std::fmt::Display for AppUidFor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client => write!(f, "dhck"),
            Self::DownloadRequest => write!(f, "dhrq"),
            Self::DownloadResult => write!(f, "dhrs"),
        }
    }
}

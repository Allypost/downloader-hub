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

        format!("{}_{}_{}", self.to_string(), ulid, time_id)
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
impl ToString for AppUidFor {
    fn to_string(&self) -> String {
        match self {
            Self::Client => "dhck",
            Self::DownloadRequest => "dhrq",
            Self::DownloadResult => "dhrs",
        }
        .to_string()
    }
}

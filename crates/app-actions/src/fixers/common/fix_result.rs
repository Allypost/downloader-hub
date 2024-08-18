use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::fix_request::FixRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    pub request: FixRequest,
    pub file_path: PathBuf,
}

impl FixResult {
    #[must_use]
    pub const fn new(request: FixRequest, file_path: PathBuf) -> Self {
        Self { request, file_path }
    }
}

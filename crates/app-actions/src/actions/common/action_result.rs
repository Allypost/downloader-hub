use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::actions::ActionRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub request: ActionRequest,
    pub file_paths: Vec<PathBuf>,
}

impl ActionResult {
    #[must_use]
    pub const fn new(request: ActionRequest, file_paths: Vec<PathBuf>) -> Self {
        Self {
            request,
            file_paths,
        }
    }
}

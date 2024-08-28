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

    #[must_use]
    pub fn from_path(request: &ActionRequest, file_path: PathBuf) -> Self {
        Self {
            request: request.clone(),
            file_paths: vec![file_path],
        }
    }

    #[must_use]
    pub fn from_paths(request: &ActionRequest, file_paths: Vec<PathBuf>) -> Self {
        Self {
            request: request.clone(),
            file_paths,
        }
    }
}

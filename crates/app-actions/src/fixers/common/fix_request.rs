use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};

use super::FixerError;

pub type FixerOptions = HashMap<String, serde_json::Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixRequest {
    pub file_path: PathBuf,
    pub options: FixerOptions,
}
impl FixRequest {
    #[must_use]
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            options: FixerOptions::new(),
        }
    }

    #[must_use]
    pub fn with_options(mut self, options: FixerOptions) -> Self {
        self.options = options;
        self
    }

    #[must_use]
    pub fn with_option(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.options.insert(key.to_string(), value.into());
        self
    }

    pub fn resolve_path(mut self) -> Result<Self, FixerError> {
        let p = self
            .file_path
            .try_resolve()
            .map_err(|e| FixerError::FailedToResolvePath(self.file_path.clone(), e))?;
        let p = p
            .canonicalize()
            .map_err(|e| FixerError::FailedToCanonicalizePath(p.to_path_buf(), e))?;

        self.file_path = p;
        Ok(self)
    }

    pub fn check_path(self) -> Result<Self, FixerError> {
        let p = &self.file_path;

        if !p.exists() {
            return Err(FixerError::FileNotFound(p.clone()));
        }

        if !p.is_file() {
            return Err(FixerError::NotAFile(p.clone()));
        }

        Ok(self)
    }

    #[must_use]
    pub fn clone_with_path(self, file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
            ..self
        }
    }
}

impl From<PathBuf> for FixRequest {
    fn from(file_path: PathBuf) -> Self {
        Self::new(file_path)
    }
}

impl From<&Path> for FixRequest {
    fn from(file_path: &Path) -> Self {
        file_path.to_path_buf().into()
    }
}

impl From<&PathBuf> for FixRequest {
    fn from(file_path: &PathBuf) -> Self {
        file_path.clone().into()
    }
}

impl From<&str> for FixRequest {
    fn from(file_path: &str) -> Self {
        file_path.to_string().into()
    }
}

impl From<String> for FixRequest {
    fn from(file_path: String) -> Self {
        Self::new(file_path.into())
    }
}

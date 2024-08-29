use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::actions::ActionRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub request: ActionRequest,
    #[serde(flatten)]
    pub data: ActionResultData,
}

impl ActionResult {
    fn new<T>(request: &ActionRequest, data: T) -> Self
    where
        T: Into<ActionResultData>,
    {
        Self {
            request: request.clone(),
            data: data.into(),
        }
    }

    #[must_use]
    pub fn path<T>(request: &ActionRequest, file_path: T) -> Self
    where
        T: Into<PathBuf>,
    {
        Self::paths(request, [file_path])
    }

    #[must_use]
    pub fn paths<T, I>(request: &ActionRequest, file_paths: T) -> Self
    where
        T: IntoIterator<Item = I>,
        I: Into<PathBuf>,
    {
        Self::new(
            request,
            file_paths.into_iter().map(Into::into).collect::<Vec<_>>(),
        )
    }

    #[must_use]
    pub fn text<T>(request: &ActionRequest, text: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::new(request, ActionResultData::Text(text.to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum ActionResultData {
    Paths(Vec<PathBuf>),
    Text(String),
}

impl From<Vec<PathBuf>> for ActionResultData {
    fn from(value: Vec<PathBuf>) -> Self {
        Self::Paths(value.into_iter().map(Into::into).collect())
    }
}

impl From<String> for ActionResultData {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&String> for ActionResultData {
    fn from(value: &String) -> Self {
        Self::Text(value.clone())
    }
}

impl From<&str> for ActionResultData {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

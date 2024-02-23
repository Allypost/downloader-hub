use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppPath {
    LocalAbsolute(PathBuf),
    None,
}

impl From<AppPath> for serde_json::Value {
    fn from(val: AppPath) -> Self {
        serde_json::json!(val)
    }
}

impl TryFrom<serde_json::Value> for AppPath {
    type Error = serde_json::Error;

    fn try_from(val: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(val)
    }
}

impl TryFrom<&serde_json::Value> for AppPath {
    type Error = serde_json::Error;

    fn try_from(val: &serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(val.clone())
    }
}

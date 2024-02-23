use std::{error::Error, fmt::Display};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ApiError(Box<dyn Error + Send + Sync>);

impl Serialize for ApiError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ApiError {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let err = String::deserialize(deserializer)?;
        Ok(Self(anyhow::anyhow!(err).into()))
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.as_ref(), f)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self(err.into())
    }
}

impl From<sea_orm::DbErr> for ApiError {
    fn from(err: sea_orm::DbErr) -> Self {
        Self(err.into())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        Self(err.into())
    }
}

impl From<String> for ApiError {
    fn from(err: String) -> Self {
        Self(anyhow::anyhow!(err).into())
    }
}

impl From<&str> for ApiError {
    fn from(err: &str) -> Self {
        Self(anyhow::anyhow!(err.to_string()).into())
    }
}

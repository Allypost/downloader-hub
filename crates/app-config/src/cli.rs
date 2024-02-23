use clap::{ArgAction, Parser};
use serde::{Deserialize, Serialize};

use crate::common;

/// A hub for downloading media from various platforms,
/// process the results and aggregate them in one place.
#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
#[clap(disable_help_flag = true)]
pub struct CliArgs {
    /// Print help
    #[clap(action = ArgAction::Help, long)]
    help: Option<bool>,

    #[command(flatten)]
    pub dependency_path: common::ProgramPathConfig,

    #[command(flatten)]
    pub endpoint: common::EndpointConfig,

    #[command(flatten)]
    pub run: common::RunConfig,

    #[command(flatten)]
    pub server: common::ServerConfig,

    #[command(flatten)]
    pub database: common::DatabaseConfig,

    #[command(flatten)]
    pub app: common::AppConfig,
}

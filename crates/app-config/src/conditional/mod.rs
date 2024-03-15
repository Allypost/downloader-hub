use clap::Args;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "server")]
pub mod server;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, Args)]
pub struct ConditionalConfig {
    #[cfg(feature = "server")]
    /// Config for the server
    #[validate]
    #[clap(flatten)]
    pub server: server::ServerConfig,

    #[cfg(feature = "cli")]
    /// Config for the CLI
    #[validate]
    #[clap(flatten)]
    pub cli: cli::CliConfig,
}

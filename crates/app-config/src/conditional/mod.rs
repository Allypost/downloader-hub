use clap::Args;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "telegram-bot")]
pub mod telegram_bot;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate, Args)]
pub struct ConditionalConfig {
    #[cfg(feature = "server")]
    /// Config for the server
    #[validate(nested)]
    #[clap(flatten)]
    pub server: server::ServerConfig,

    #[cfg(feature = "cli")]
    /// Config for the CLI
    #[validate(nested)]
    #[clap(flatten)]
    pub cli: cli::CliConfig,

    #[cfg(feature = "telegram-bot")]
    /// Config for the Telegram bot
    #[validate(nested)]
    #[clap(flatten)]
    pub telegram_bot: telegram_bot::TelegramBotConfig,
}

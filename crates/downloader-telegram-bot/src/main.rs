pub(crate) mod bot;
pub(crate) mod queue;

use app_config::Config;
use app_logger::{debug, error};
use queue::TaskQueueProcessor;

#[tokio::main]
async fn main() {
    let loaded_dotenv = dotenvy::dotenv();

    app_logger::init();

    match loaded_dotenv {
        Ok(loaded_dotenv) => {
            debug!(path = ?loaded_dotenv, "Loaded dotenv file");
        }
        Err(e) if e.not_found() => {
            debug!("No dotenv file found");
        }
        Err(e) => {
            error!("Failed to load dotenv file: {e:?}");
            panic!("Failed to load dotenv file: {e:?}");
        }
    }

    debug!(config = ?*Config::global(), "Running with config");

    tokio::task::spawn(TaskQueueProcessor::run());

    bot::run().await.expect("Failed to run server");
}

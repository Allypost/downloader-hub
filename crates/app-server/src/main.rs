use app_config::Config;

use crate::{
    db::AppDb,
    queue::{processor::TaskQueueProcessor, TaskQueue},
};

mod db;
mod queue;
mod server;
mod service;

#[tokio::main]
async fn main() {
    let loaded_dotenv = dotenvy::dotenv();

    app_logger::init();

    match loaded_dotenv {
        Ok(loaded_dotenv) => {
            app_logger::debug!(path = ?loaded_dotenv, "Loaded dotenv file");
        }
        Err(e) if e.not_found() => {
            app_logger::debug!("No dotenv file found");
        }
        Err(e) => {
            app_logger::error!("Failed to load dotenv file: {e:?}");
            panic!("Failed to load dotenv file: {e:?}");
        }
    }

    app_logger::debug!(config = ?*Config::global(), "Running with config");

    AppDb::init().await.expect("Failed to initialize database");

    TaskQueue::init().await.expect("Failed to initialize queue");

    tokio::task::spawn(TaskQueueProcessor::run());

    server::run().await.expect("Failed to run server");
}

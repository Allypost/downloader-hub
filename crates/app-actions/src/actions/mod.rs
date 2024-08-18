mod common;
pub mod handlers;

use std::fmt::Debug;

pub use common::{
    action_error::ActionError,
    action_request::{ActionOptions, ActionRequest},
    action_result::ActionResult,
};

#[async_trait::async_trait]
pub trait Action: Debug + Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    async fn can_run(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    async fn can_run_for(&self, req: &ActionRequest) -> bool {
        true
    }

    async fn run(&self, req: &ActionRequest) -> Result<ActionResult, ActionError>;
}

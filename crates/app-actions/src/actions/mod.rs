mod common;
pub mod handlers;

use std::fmt::Debug;

pub use common::{
    action_error::ActionError,
    action_request::{ActionOptions, ActionRequest},
    action_result::{ActionResult, ActionResultData},
};
pub use handlers::AVAILABLE_ACTIONS;

#[async_trait::async_trait]
#[typetag::serde(tag = "$action")]
pub trait Action: Debug + Send + Sync {
    fn name(&self) -> &'static str {
        self.typetag_name()
    }

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

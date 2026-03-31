use serde_json::Value;

use crate::{async_trait, error::BoxError, scheduler::context::JobExecutionContext};

#[async_trait]
pub trait ScheduledJobHandler: Send + Sync {
    fn task_key(&self) -> &'static str;

    async fn execute(
        &self,
        context: JobExecutionContext,
        payload: Option<Value>,
    ) -> Result<(), BoxError>;
}

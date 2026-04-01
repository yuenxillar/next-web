use serde_json::Value;

use crate::{async_trait, error::BoxError, scheduler::context::JobExecutionContext};

#[async_trait]
pub trait ScheduledJobHandler
where
    Self: Send + Sync,
{
    fn id(&self) -> &str;

    async fn execute(
        &self,
        ctx: JobExecutionContext,
        payload: Option<Value>,
    ) -> Result<(), BoxError>;
}

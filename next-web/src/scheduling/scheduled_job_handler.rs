use serde_json::Value;

use next_web_core::{async_trait, error::BoxError};

use super::context::JobExecutionContext;

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

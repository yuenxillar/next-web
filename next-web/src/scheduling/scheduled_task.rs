use next_web_core::async_trait;

use super::{context::JobExecutionContext, schedule_type::ScheduleType};

/// ScheduledTask is a trait that defines the behavior of an application job.
#[async_trait]
pub trait ScheduledTask
where
    Self: Send + Sync,
{
    fn schedule(&self) -> ScheduleType;

    async fn execute(&self, context: JobExecutionContext);
}

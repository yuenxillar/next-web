use crate::{
    core::job::{context::job_execution_context::JobExecutionContext, schedule_type::ScheduleType},
    error::BoxError,
};
use async_trait::async_trait;

/// ApplicationJob is a trait that defines the behavior of an application job.

#[async_trait]
pub trait ApplicationJob: Send + Sync {
    
    fn schedule(&self) -> ScheduleType;

    async fn execute(&self, context: JobExecutionContext) -> Result<(), BoxError>;
}
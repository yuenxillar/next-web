use async_trait::async_trait;

use crate::{error::BoxError, scheduler::PersistedScheduledJob};

#[async_trait]
pub trait ScheduledJobStore
where
    Self: Send + Sync,
{
    async fn save(&self, job: PersistedScheduledJob) -> Result<(), BoxError>;

    async fn delete(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError>;
}

use next_web_core::{async_trait, error::BoxError};

use super::persisted_job::PersistedScheduledJob;

#[async_trait]
pub trait ScheduledJobStore
where
    Self: Send + Sync,
{
    async fn save(&self, job: PersistedScheduledJob) -> Result<(), BoxError>;

    async fn delete(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError>;
}

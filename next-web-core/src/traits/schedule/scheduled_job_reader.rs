use async_trait::async_trait;

use crate::{error::BoxError, scheduler::PersistedScheduledJob};

#[async_trait]
pub trait ScheduledJobReader
where
    Self: Send + Sync,
{
    async fn read(&self) -> Result<Vec<PersistedScheduledJob>, BoxError>;

    async fn find(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError>;

    async fn exists(&self, id: &str) -> Result<bool, BoxError> {
        Ok(self.find(id).await?.is_some())
    }
}

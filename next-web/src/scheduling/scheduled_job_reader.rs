use next_web_core::{async_trait, error::BoxError};

use super::persisted_job::PersistedScheduledJob;

#[async_trait]
pub trait ScheduledJobReader
where
    Self: Send + Sync,
{
    /// Reads every job the repository holds.
    ///
    /// The jobs a task was disabled with are returned as well, which is what
    /// lets an application show and change the state of a task.
    async fn read(&self) -> Result<Vec<PersistedScheduledJob>, BoxError>;

    /// Reads the jobs that run, which is what a scheduler restores.
    async fn read_enabled(&self) -> Result<Vec<PersistedScheduledJob>, BoxError> {
        Ok(self
            .read()
            .await?
            .into_iter()
            .filter(|job| job.enabled)
            .collect())
    }

    /// Reads the job registered under `id`, when there is one.
    async fn find(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError>;

    /// Returns whether a job is registered under `id`.
    async fn exists(&self, id: &str) -> Result<bool, BoxError> {
        Ok(self.find(id).await?.is_some())
    }
}

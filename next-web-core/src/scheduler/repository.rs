use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{async_trait, error::BoxError, scheduler::persisted_job::PersistedScheduledJob};

#[async_trait]
pub trait ScheduledJobReader: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError>;

    async fn list(&self) -> Result<Vec<PersistedScheduledJob>, BoxError>;

    async fn list_enabled(&self) -> Result<Vec<PersistedScheduledJob>, BoxError> {
        let jobs = self.list().await?;
        Ok(jobs.into_iter().filter(|job| job.enabled).collect())
    }

    async fn exists(&self, id: &str) -> Result<bool, BoxError> {
        Ok(self.find_by_id(id).await?.is_some())
    }

    async fn count(&self) -> Result<usize, BoxError> {
        Ok(self.list().await?.len())
    }
}

#[async_trait]
pub trait ScheduledJobStore: Send + Sync {
    async fn save(&self, job: PersistedScheduledJob) -> Result<(), BoxError>;

    async fn delete(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError>;
}

pub trait ScheduledJobRepository
where
    Self: ScheduledJobReader + ScheduledJobStore,
{
}

impl<T> ScheduledJobRepository for T where T: ScheduledJobReader + ScheduledJobStore {}

#[derive(Clone, Default)]
pub struct InMemoryScheduledJobRepository {
    jobs: Arc<RwLock<HashMap<String, PersistedScheduledJob>>>,
}

impl InMemoryScheduledJobRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ScheduledJobReader for InMemoryScheduledJobRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError> {
        Ok(self.jobs.read().await.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<PersistedScheduledJob>, BoxError> {
        Ok(self.jobs.read().await.values().cloned().collect())
    }
}

#[async_trait]
impl ScheduledJobStore for InMemoryScheduledJobRepository {
    async fn save(&self, job: PersistedScheduledJob) -> Result<(), BoxError> {
        self.jobs.write().await.insert(job.id.clone(), job);
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError> {
        Ok(self.jobs.write().await.remove(id))
    }
}

#[cfg(test)]
mod tests {
    use crate::scheduler::{
        PersistedScheduledJob,
        repository::{InMemoryScheduledJobRepository, ScheduledJobReader, ScheduledJobStore},
        schedule_type::{ScheduleType, WithArgs},
    };

    fn one_shot_schedule() -> ScheduleType {
        ScheduleType::OneShot(WithArgs {
            initial_delay: Some(1),
            time_unit: Some("s".to_string()),
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn in_memory_repository_reads_and_filters_enabled_jobs() {
        let repository = InMemoryScheduledJobRepository::new();

        repository
            .save(PersistedScheduledJob::new(
                "enabled",
                "task.a",
                one_shot_schedule(),
            ))
            .await
            .unwrap();
        repository
            .save(
                PersistedScheduledJob::new("disabled", "task.b", one_shot_schedule())
                    .enabled(false),
            )
            .await
            .unwrap();

        assert!(repository.find_by_id("enabled").await.unwrap().is_some());
        assert_eq!(repository.count().await.unwrap(), 2);
        assert_eq!(repository.list_enabled().await.unwrap().len(), 1);
    }
}

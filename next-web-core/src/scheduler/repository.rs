use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    async_trait,
    error::BoxError,
    scheduler::persisted_job::PersistedScheduledJob,
    traits::schedule::{
        scheduled_job_reader::ScheduledJobReader, scheduled_job_store::ScheduledJobStore,
    },
};

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
    async fn find(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError> {
        Ok(self.jobs.read().await.get(id).cloned())
    }

    async fn read(&self) -> Result<Vec<PersistedScheduledJob>, BoxError> {
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
    use crate::{
        scheduler::{
            InMemoryScheduledJobRepository, PersistedScheduledJob,
            schedule_type::{ScheduleType, WithArgs},
        },
        traits::schedule::{
            scheduled_job_reader::ScheduledJobReader, scheduled_job_store::ScheduledJobStore,
        },
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

        assert!(repository.find("enabled").await.unwrap().is_some());
        assert_eq!(repository.read().await.unwrap().len(), 1);
    }
}

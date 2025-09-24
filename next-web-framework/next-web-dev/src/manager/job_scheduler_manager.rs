use std::{collections::HashSet, sync::Arc};

use next_web_core::{
    error::BoxError,
    scheduler::{context::JobExecutionContext, schedule_type::ScheduleType},
    traits::{schedule::scheduled_task::ScheduledTask, singleton::Singleton},
};
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::warn;

use crate::autoregister::scheduler_autoregister::AnJob;

#[derive(Clone)]
pub struct JobSchedulerManager {
    ids: Arc<RwLock<HashSet<u128>>>,
    scheduler: JobScheduler,
    context: JobExecutionContext,
}

impl Singleton for JobSchedulerManager {}

impl JobSchedulerManager {
    pub async fn with_channel_size(size: usize) -> Self {
        Self {
            ids: Arc::new(RwLock::new(HashSet::new())),
            context: JobExecutionContext::default(),
            scheduler: JobScheduler::new_with_channel_size(size).await.unwrap(),
        }
    }

    pub async fn add(&self, job: AnJob) -> Result<(), JobSchedulerError> {
        let job = Self::pack(job)?;
        let uid = self.scheduler.add(job).await?;
        self.ids.write().await.insert(uid.as_u128());

        Ok(())
    }

    pub async fn add_job(&self, job: Arc<dyn ScheduledTask>) {
        let jjb = Self::generate_job(job, self.context.clone());
        if jjb.is_err() {
            warn!(
                "JobSchedulerManager failed to add job, error: {}",
                jjb.err().unwrap()
            );
            return;
        }
    }

    pub async fn remove(&self, guid: Vec<u8>) {
        match guid.try_into() {
            Ok(uid) => self.scheduler.remove(&uid).await.ok(),
            Err(e) => {
                warn!("JobSchedulerManager failed to remove job, error: {}", e);
                None
            }
        };
    }

    pub async fn exists(&self, uid: u128) -> bool {
        self.ids.read().await.contains(&uid)
    }

    pub async fn count(&self) -> usize {
        self.ids.read().await.len()
    }

    pub async fn start(&mut self) {
        // Add code to be run during/after shutdown
        self.scheduler.set_shutdown_handler(Box::new(|| {
            Box::pin(async move {
                println!("Shut down done");
            })
        }));

        // Start the scheduler
        self.scheduler.start().await.unwrap();
    }

    fn pack(job: AnJob) -> Result<Job, JobSchedulerError> {
        match job {
            AnJob::Async(run) => Job::new_repeated_async(
                std::time::Duration::from_millis(1000),
                move |_uid, _lock| run(),
            ),
            AnJob::Sync(run) => {
                Job::new_repeated(std::time::Duration::from_millis(900), move |_uid, _lock| {
                    run()
                })
            }
        }
    }

    fn generate_job(
        job: Arc<dyn ScheduledTask>,
        context: JobExecutionContext,
    ) -> Result<Job, BoxError> {
        let schedule = job.schedule();

        let jjb = match schedule {
            ScheduleType::Cron(cron) => Job::new_cron_job_async(cron, move |_uid, _lock| {
                Box::pin({
                    let var1 = job.clone();
                    let var2 = context.clone();
                    async move {
                        var1.execute(var2).await;
                    }
                })
            }),
            ScheduleType::Repeated(interval) => Job::new_repeated_async(
                std::time::Duration::from_secs(interval),
                move |_uid, _lock| {
                    Box::pin({
                        let var1 = job.clone();
                        let var2 = context.clone();
                        async move {
                            var1.execute(var2).await;
                        }
                    })
                },
            ),
            ScheduleType::OneShot(interval) => Job::new_one_shot_async(
                std::time::Duration::from_secs(interval),
                move |_uid, _lock| {
                    Box::pin({
                        let var1 = job.clone();
                        let var2 = context.clone();
                        async move {
                            var1.execute(var2).await;
                        }
                    })
                },
            ),
            ScheduleType::OneShotAtInstant(instant) => {
                Job::new_one_shot_at_instant_async(instant, move |_uid, _lock| {
                    Box::pin({
                        let var1 = job.clone();
                        let var2 = context.clone();
                        async move {
                            var1.execute(var2).await;
                        }
                    })
                })
            }
        };

        Ok(jjb?)
    }
}

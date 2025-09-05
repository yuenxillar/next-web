use std::sync::Arc;

use flume::Sender;
use hashbrown::HashSet;
use next_web_core::{
    error::BoxError, traits::{
        job::{
            application_job::ApplicationJob, context::job_execution_context::JobExecutionContext,
            schedule_type::ScheduleType,
        }, singleton::Singleton
    }
};
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, warn};

#[derive(Clone)]
pub struct JobSchedulerManager {
    ids: Arc<RwLock<HashSet<Vec<u8>>>>,
    jobs: Arc<RwLock<Vec<Job>>>,
    tx: Option<Sender<SchedulerEvent>>,
    context: JobExecutionContext,
}

impl Singleton  for JobSchedulerManager {}


impl JobSchedulerManager {
    pub fn new() -> Self {
        Self {
            ids: Arc::new(RwLock::new(HashSet::new())),
            jobs: Arc::new(RwLock::new(Vec::new())),
            context: JobExecutionContext::default(),
            tx: None,
        }
    }

    pub async fn add_job(&self, job: Arc<dyn ApplicationJob>) {
        let jjb = Self::generate_job(job, self.context.clone());
        if jjb.is_err() {
            warn!(
                "JobSchedulerManager failed to add job, error: {}",
                jjb.err().unwrap()
            );
            return;
        }

        if let Some(sender) = &self.tx {
            sender
                .send_async(SchedulerEvent::AddJob(jjb.unwrap()))
                .await
                .ok();
        }
    }

    pub async fn remove_job(&self, guid: Vec<u8>) {
        if let Some(sender) = &self.tx {
            sender
                .send_async(SchedulerEvent::RemoveJob(guid))
                .await
                .ok();
        }
    }

    pub async fn exists(&self, guid: Vec<u8>) -> bool {
        self.ids.read().await.contains(&guid)
    }

    pub(crate) fn start(&mut self) {
        let jobs = self.jobs.clone();
        let ids = self.ids.clone();

        let (tx, rx) = flume::bounded(1024);
        self.tx = Some(tx);


        tokio::spawn(async move {
            let mut scheduler = JobScheduler::new().await.unwrap();
            let jobs = jobs.read().await;
            for job in jobs.iter() {
                let job_id = scheduler.add(job.clone()).await.unwrap();
                if ids.write().await.insert(job_id.as_bytes().to_vec()) {
                    info!("Job: {} added successfully!", job_id)
                }
            }

            // Add code to be run during/after shutdown
            scheduler.set_shutdown_handler(Box::new(|| {
                Box::pin(async move {
                    println!("Shut down done");
                })
            }));

            scheduler.start().await.unwrap();
            // spawn a task to listen for job removal

            tokio::spawn(async move {
                while let Ok(event) = rx.recv() {
                    match event {
                        SchedulerEvent::AddJob(job) => {
                            let job_id = scheduler.add(job).await.unwrap();
                            let _ = ids.write().await.insert(job_id.as_bytes().to_vec());
                        }
                        SchedulerEvent::RemoveJob(guid) => {
                            if let Ok(uuid) = guid.try_into() {
                                let _ = scheduler
                                    .remove(&uuid)
                                    .await
                                    .map(|_| info!("Job removed successfully;"));
                            };
                        }
                        SchedulerEvent::Shutdown => {
                            scheduler.shutdown().await.unwrap();
                        }
                    }
                }
            });
        });
    }

    pub async fn count(&self) -> usize {
        self.ids.read().await.len()
    }

    fn generate_job(
        job: Arc<dyn ApplicationJob>,
        context: JobExecutionContext,
    ) -> Result<Job, BoxError> {
        let schedule = job.schedule();

        let jjb = match schedule {
            ScheduleType::Cron(cron) => Job::new_cron_job_async(cron, move |_uid, _lock| {
                Box::pin({
                    let var1 = job.clone();
                    let var2 = context.clone();
                    async move {
                        var1.execute(var2).await.unwrap();
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
                            var1.execute(var2).await.unwrap();
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
                            var1.execute(var2).await.unwrap();
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
                            var1.execute(var2).await.unwrap();
                        }
                    })
                })
            }
        };

        Ok(jjb?)
    }
}

#[derive(Clone)]
pub enum SchedulerEvent {
    AddJob(Job),
    RemoveJob(Vec<u8>),
    Shutdown,
}

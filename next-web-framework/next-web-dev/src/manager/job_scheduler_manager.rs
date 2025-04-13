use std::sync::Arc;

use flume::Sender;
use hashbrown::HashSet;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

#[derive(Clone)]
pub struct JobSchedulerManager {
    ids: Arc<Mutex<HashSet<Vec<u8>>>>,
    jobs: Arc<Mutex<Vec<Job>>>,
    tx: Option<Sender<SchedulerEvent>>,
}

impl JobSchedulerManager {
    pub fn new() -> Self {
        Self {
            ids: Arc::new(Mutex::new(HashSet::new())),
            jobs: Arc::new(Mutex::new(Vec::new())),
            tx: None,
        }
    }

    pub async fn add_job(&self, job: Job) {
        if let Some(sender) = &self.tx {
            sender.send_async(SchedulerEvent::AddJob(job)).await.ok();
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

    pub async fn check_job_exists(&self, guid: Vec<u8>) -> bool {
        self.ids.lock().await.contains(&guid)
    }

    pub fn start(&mut self) {
        let jobs = self.jobs.clone();
        let ids = self.ids.clone();
        let (tx, rx) = flume::unbounded();
        self.tx = Some(tx);

        tokio::spawn(async move {
            let mut scheduler = JobScheduler::new().await.unwrap();
            let jobs = jobs.lock().await;
            for job in jobs.iter() {
                let job_id = scheduler.add(job.clone()).await.unwrap();
                let _ = ids
                    .try_lock()
                    .map(|mut ids| ids.insert(job_id.as_bytes().to_vec()))
                    .map(|_| info!("Job: {} added successfully!", job_id));
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
                            let _ = ids
                                .try_lock()
                                .map(|mut ids| ids.insert(job_id.as_bytes().to_vec()));
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

    pub async fn job_count(&self) -> usize {
        self.ids.lock().await.len()
    }
}

#[derive(Clone)]
pub enum SchedulerEvent {
    AddJob(Job),
    RemoveJob(Vec<u8>),
    Shutdown,
}

pub trait ApplicationJob: Send + Sync {
    fn gen_job(&self) -> Job;
}

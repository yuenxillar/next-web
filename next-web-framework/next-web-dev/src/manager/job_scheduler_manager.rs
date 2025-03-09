use std::sync::{Arc, Mutex};

use hashbrown::HashSet;
use tokio::sync::mpsc::Sender;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

#[derive(Clone)]
pub struct JobSchedulerManager {
    ids: Arc<Mutex<HashSet<Vec<u8>>>>,
    jobs: Vec<Job>,
    tx: Option<Sender<SchedulerEvent>>,
}

impl JobSchedulerManager {
    pub fn new() -> Self {
        Self {
            ids: Arc::new(Mutex::new(HashSet::new())),
            jobs: Vec::new(),
            tx: None,
        }
    }

    pub fn add_job(&mut self, job: Job) {
        self.jobs.push(job);
    }

    pub fn start(&mut self) {
        let jobs = self.jobs.clone();
        let ids = self.ids.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        self.tx = Some(tx);

        tokio::spawn(async move {
            let mut scheduler = JobScheduler::new().await.unwrap();

            for job in jobs {
                let job_id = scheduler.add(job).await.unwrap();
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
                while let Some(event) = rx.recv().await {
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

    pub fn job_count(&self) -> usize {
        self.ids.try_lock().map(|ids| ids.len()).unwrap_or(0)
    }
}

#[derive(Clone)]
pub enum SchedulerEvent {
    AddJob(Job),
    RemoveJob(Vec<u8>),
    Shutdown,
}

pub trait ApplicationJob: Send + Sync {
    fn generate_job(&self) -> Job;
}

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::Arc,
};

use next_web_core::{
    error::BoxError,
    scheduler::{
        context::JobExecutionContext,
        persisted_job::PersistedScheduledJob,
        repository::{
            InMemoryScheduledJobRepository, ScheduledJobReader, ScheduledJobRepository,
            ScheduledJobStore,
        },
        schedule_type::{ScheduleType, WithArgs},
        ScheduledJobRegistry,
    },
    traits::singleton::Singleton,
    util::time::TimeUnit,
};
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::{error, warn};
use uuid::Uuid;

type JobFuture = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static>;
type JobFunction = Box<dyn Fn() + Send + Sync + 'static>;

pub enum BoxedJob {
    Async((ScheduleType, JobFuture)),
    Sync((ScheduleType, JobFunction)),
}

#[derive(Clone)]
pub struct JobSchedulerManager {
    static_ids: Arc<RwLock<HashSet<Uuid>>>,
    runtime_ids: Arc<RwLock<HashMap<String, Uuid>>>,
    scheduler: JobScheduler,
    reader: Arc<dyn ScheduledJobReader>,
    store: Arc<dyn ScheduledJobStore>,
    registry: ScheduledJobRegistry,
}

impl Singleton for JobSchedulerManager {}

#[derive(Clone)]
struct RepositoryReaderAdapter {
    repository: Arc<dyn ScheduledJobRepository>,
}

#[next_web_core::async_trait]
impl ScheduledJobReader for RepositoryReaderAdapter {
    async fn find_by_id(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError> {
        self.repository.find_by_id(id).await
    }

    async fn list(&self) -> Result<Vec<PersistedScheduledJob>, BoxError> {
        self.repository.list().await
    }
}

#[derive(Clone)]
struct RepositoryStoreAdapter {
    repository: Arc<dyn ScheduledJobRepository>,
}

#[next_web_core::async_trait]
impl ScheduledJobStore for RepositoryStoreAdapter {
    async fn save(&self, job: PersistedScheduledJob) -> Result<(), BoxError> {
        self.repository.save(job).await
    }

    async fn delete(&self, id: &str) -> Result<Option<PersistedScheduledJob>, BoxError> {
        self.repository.delete(id).await
    }
}

impl JobSchedulerManager {
    pub async fn with_channel_size(size: usize) -> Self {
        let repository = Arc::new(InMemoryScheduledJobRepository::new());
        let registry = ScheduledJobRegistry::default();
        Self::with_channel_size_and_repository(size, repository, registry).await
    }

    pub async fn with_channel_size_and_repository<R>(
        size: usize,
        repository: Arc<R>,
        registry: ScheduledJobRegistry,
    ) -> Self
    where
        R: ScheduledJobRepository + 'static,
    {
        let reader: Arc<dyn ScheduledJobReader> = repository.clone();
        let store: Arc<dyn ScheduledJobStore> = repository;

        Self::with_channel_size_and_components(size, reader, store, registry).await
    }

    pub async fn with_channel_size_from_repository(
        size: usize,
        repository: Arc<dyn ScheduledJobRepository>,
        registry: ScheduledJobRegistry,
    ) -> Self {
        let reader: Arc<dyn ScheduledJobReader> = Arc::new(RepositoryReaderAdapter {
            repository: repository.clone(),
        });
        let store: Arc<dyn ScheduledJobStore> = Arc::new(RepositoryStoreAdapter { repository });

        Self::with_channel_size_and_components(size, reader, store, registry).await
    }

    pub async fn with_channel_size_and_components(
        size: usize,
        reader: Arc<dyn ScheduledJobReader>,
        store: Arc<dyn ScheduledJobStore>,
        registry: ScheduledJobRegistry,
    ) -> Self {
        Self {
            static_ids: Arc::new(RwLock::new(HashSet::new())),
            runtime_ids: Arc::new(RwLock::new(HashMap::new())),
            scheduler: JobScheduler::new_with_channel_size(size).await.unwrap(),
            reader,
            store,
            registry,
        }
    }

    pub fn registry(&self) -> ScheduledJobRegistry {
        self.registry.clone()
    }

    pub async fn add(&self, job: BoxedJob) -> Result<(), JobSchedulerError> {
        self.add_static(job).await
    }

    pub async fn add_static(&self, job: BoxedJob) -> Result<(), JobSchedulerError> {
        let job = Self::pack(job)?;
        let uid = job.guid();
        self.scheduler.add(job).await?;
        self.static_ids.write().await.insert(uid);

        Ok(())
    }

    pub async fn create_persisted(
        &self,
        job: PersistedScheduledJob,
        context: JobExecutionContext,
    ) -> Result<(), BoxError> {
        if self.reader.exists(&job.id).await? {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("persisted scheduled job already exists: {}", job.id),
            )));
        }

        self.store.save(job.clone()).await?;

        if job.enabled {
            if let Err(error) = self.schedule_persisted(job.clone(), context).await {
                if let Err(delete_error) = self.store.delete(&job.id).await {
                    warn!(
                        job_id = %job.id,
                        error = %delete_error,
                        "Persisted scheduled job rollback failed after schedule error"
                    );
                }
                return Err(error);
            }
        }

        Ok(())
    }

    pub async fn schedule_persisted(
        &self,
        job: PersistedScheduledJob,
        context: JobExecutionContext,
    ) -> Result<Uuid, BoxError> {
        if !job.enabled {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("persisted scheduled job is disabled: {}", job.id),
            )));
        }

        let handler = self.registry.get(&job.task_key).ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "scheduled job handler not found for task_key: {}",
                    job.task_key
                ),
            )) as BoxError
        })?;

        if let Some(runtime_id) = self.runtime_ids.read().await.get(&job.id).copied() {
            self.scheduler
                .remove(&runtime_id)
                .await
                .map_err(|error| -> BoxError { Box::new(error) })?;
            self.runtime_ids.write().await.remove(&job.id);
        }

        let reader = self.reader.clone();
        let store = self.store.clone();
        let runtime_ids = self.runtime_ids.clone();
        let schedule = job.schedule.clone();
        let job_id = job.id.clone();
        let task_key = job.task_key.clone();
        let payload = job.payload.clone();
        let one_shot = matches!(schedule, ScheduleType::OneShot(_));

        let runtime_job = Self::pack_persisted_job(schedule, move |runtime_uid, scheduler| {
            let handler = handler.clone();
            let reader = reader.clone();
            let store = store.clone();
            let runtime_ids = runtime_ids.clone();
            let context = context.clone();
            let job_id = job_id.clone();
            let task_key = task_key.clone();
            let payload = payload.clone();

            Box::pin(async move {
                if let Err(exec_error) = handler.execute(context, payload).await {
                    error!(
                        job_id = %job_id,
                        task_key = %task_key,
                        error = %exec_error,
                        "Persisted scheduled job execution failed"
                    );
                    return;
                }

                match reader.find_by_id(&job_id).await {
                    Ok(Some(mut persisted_job)) => {
                        persisted_job.mark_ran_now();
                        if let Err(save_error) = store.save(persisted_job).await {
                            warn!(
                                job_id = %job_id,
                                error = %save_error,
                                "Persisted scheduled job metadata update failed"
                            );
                        }
                    }
                    Ok(None) => {}
                    Err(read_error) => {
                        warn!(
                            job_id = %job_id,
                            error = %read_error,
                            "Persisted scheduled job lookup failed after execution"
                        );
                    }
                }

                if one_shot {
                    if let Err(remove_error) = scheduler.remove(&runtime_uid).await {
                        warn!(
                            job_id = %job_id,
                            runtime_uid = %runtime_uid,
                            error = %remove_error,
                            "Persisted one-shot runtime removal failed"
                        );
                    }

                    runtime_ids.write().await.remove(&job_id);

                    if let Err(delete_error) = store.delete(&job_id).await {
                        warn!(
                            job_id = %job_id,
                            error = %delete_error,
                            "Persisted one-shot repository cleanup failed"
                        );
                    }
                }
            })
        })
        .map_err(|error| -> BoxError { Box::new(error) })?;

        let uid = runtime_job.guid();
        self.scheduler
            .add(runtime_job)
            .await
            .map_err(|error| -> BoxError { Box::new(error) })?;
        self.runtime_ids.write().await.insert(job.id, uid);

        Ok(uid)
    }

    pub async fn restore_all(&self, context: JobExecutionContext) -> Result<(), BoxError> {
        for job in self.reader.list_enabled().await? {
            if self.registry.get(&job.task_key).is_none() {
                warn!(
                    job_id = %job.id,
                    task_key = %job.task_key,
                    "Persisted scheduled job skipped because handler is not registered"
                );
                continue;
            }

            if let Err(error) = self.schedule_persisted(job.clone(), context.clone()).await {
                warn!(
                    job_id = %job.id,
                    task_key = %job.task_key,
                    error = %error,
                    "Persisted scheduled job restore failed"
                );
            }
        }

        Ok(())
    }

    pub async fn remove_persisted(
        &self,
        id: &str,
    ) -> Result<Option<PersistedScheduledJob>, BoxError> {
        let runtime_id = { self.runtime_ids.write().await.remove(id) };
        if let Some(runtime_id) = runtime_id {
            self.scheduler
                .remove(&runtime_id)
                .await
                .map_err(|error| -> BoxError { Box::new(error) })?;
        }

        self.store.delete(id).await
    }

    pub async fn exists_persisted(&self, id: &str) -> Result<bool, BoxError> {
        self.reader.exists(id).await
    }

    pub async fn count_persisted(&self) -> Result<usize, BoxError> {
        self.reader.count().await
    }

    pub async fn remove(&self, guid: Vec<u8>) {
        match Uuid::from_slice(&guid) {
            Ok(uid) => {
                if let Err(error) = self.scheduler.remove(&uid).await {
                    warn!(
                        runtime_uid = %uid,
                        error = %error,
                        "JobSchedulerManager failed to remove runtime job"
                    );
                    return;
                }

                self.static_ids.write().await.remove(&uid);
                self.runtime_ids
                    .write()
                    .await
                    .retain(|_, value| *value != uid);
            }
            Err(error) => {
                warn!(
                    error = %error,
                    "JobSchedulerManager failed to parse runtime uid for removal"
                );
            }
        }
    }

    pub async fn exists(&self, uid: u128) -> bool {
        let runtime_uid = Uuid::from_u128(uid);
        if self.static_ids.read().await.contains(&runtime_uid) {
            return true;
        }

        self.runtime_ids
            .read()
            .await
            .values()
            .any(|value| *value == runtime_uid)
    }

    pub async fn count(&self) -> usize {
        self.static_ids.read().await.len() + self.runtime_ids.read().await.len()
    }

    pub async fn start(&mut self) -> Result<(), JobSchedulerError> {
        self.scheduler.set_shutdown_handler(Box::new(|| {
            Box::pin(async move {
                println!("Scheduler Done.");
            })
        }));

        self.scheduler.start().await?;

        Ok(())
    }

    fn pack(job: BoxedJob) -> Result<Job, JobSchedulerError> {
        match job {
            BoxedJob::Async((schedule, run)) => {
                Self::build_async_job(schedule, move |_uid, _lock| run())
            }
            BoxedJob::Sync((schedule, run)) => {
                Self::build_sync_job(schedule, move |_uid, _lock| run())
            }
        }
    }

    fn pack_persisted_job<T>(schedule: ScheduleType, run: T) -> Result<Job, JobSchedulerError>
    where
        T: 'static,
        T: FnMut(Uuid, JobScheduler) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
    {
        Self::build_async_job(schedule, run)
    }

    fn build_async_job<T>(schedule: ScheduleType, run: T) -> Result<Job, JobSchedulerError>
    where
        T: 'static,
        T: FnMut(Uuid, JobScheduler) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
    {
        match schedule {
            ScheduleType::Cron(args) => Self::build_async_cron_job(args, run),
            ScheduleType::FixedRate(args) => {
                Job::new_repeated_async(Self::duration_from_fixed_rate(&args)?, run)
            }
            ScheduleType::OneShot(args) => {
                Job::new_one_shot_async(Self::duration_from_initial_delay(&args)?, run)
            }
        }
    }

    fn build_sync_job<T>(schedule: ScheduleType, run: T) -> Result<Job, JobSchedulerError>
    where
        T: 'static,
        T: FnMut(Uuid, JobScheduler) + Send + Sync,
    {
        match schedule {
            ScheduleType::Cron(args) => Self::build_sync_cron_job(args, run),
            ScheduleType::FixedRate(args) => {
                Job::new_repeated(Self::duration_from_fixed_rate(&args)?, run)
            }
            ScheduleType::OneShot(args) => {
                Job::new_one_shot(Self::duration_from_initial_delay(&args)?, run)
            }
        }
    }

    fn build_async_cron_job<T>(args: WithArgs, run: T) -> Result<Job, JobSchedulerError>
    where
        T: 'static,
        T: FnMut(Uuid, JobScheduler) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
    {
        let schedule = args
            .cron
            .as_deref()
            .ok_or(JobSchedulerError::ParseSchedule)?;

        match args.timezone.as_deref() {
            Some(timezone) if timezone.eq_ignore_ascii_case("local") => {
                Job::new_cron_job_async_tz(schedule, chrono::Local, run)
            }
            Some(timezone) if timezone.eq_ignore_ascii_case("utc") => {
                Job::new_cron_job_async_tz(schedule, chrono::Utc, run)
            }
            Some(timezone) => Job::new_cron_job_async_tz(
                schedule,
                timezone.parse::<chrono_tz::Tz>().unwrap_or(chrono_tz::UTC),
                run,
            ),
            None => Job::new_cron_job_async(schedule, run),
        }
    }

    fn build_sync_cron_job<T>(args: WithArgs, run: T) -> Result<Job, JobSchedulerError>
    where
        T: 'static,
        T: FnMut(Uuid, JobScheduler) + Send + Sync,
    {
        let schedule = args
            .cron
            .as_deref()
            .ok_or(JobSchedulerError::ParseSchedule)?;

        match args.timezone.as_deref() {
            Some(timezone) if timezone.eq_ignore_ascii_case("local") => {
                Job::new_tz(schedule, chrono::Local, run)
            }
            Some(timezone) if timezone.eq_ignore_ascii_case("utc") => {
                Job::new_tz(schedule, chrono::Utc, run)
            }
            Some(timezone) => Job::new_tz(
                schedule,
                timezone.parse::<chrono_tz::Tz>().unwrap_or(chrono_tz::UTC),
                run,
            ),
            None => Job::new_cron_job::<_, _, ()>(schedule, run),
        }
    }

    fn duration_from_fixed_rate(args: &WithArgs) -> Result<std::time::Duration, JobSchedulerError> {
        Self::duration_from_value(args.fixed_rate, args.time_unit.as_deref())
    }

    fn duration_from_initial_delay(
        args: &WithArgs,
    ) -> Result<std::time::Duration, JobSchedulerError> {
        Self::duration_from_value(args.initial_delay, args.time_unit.as_deref())
    }

    fn duration_from_value(
        value: Option<u64>,
        time_unit: Option<&str>,
    ) -> Result<std::time::Duration, JobSchedulerError> {
        let value = value.ok_or(JobSchedulerError::ParseSchedule)?;

        Ok(time_unit
            .map(|unit| unit.parse::<TimeUnit>().unwrap_or(TimeUnit::Milliseconds))
            .map(|unit| unit.to_duration(value))
            .unwrap_or(std::time::Duration::from_millis(value)))
    }
}

#[cfg(all(test, feature = "enable-scheduling"))]
mod tests {
    use std::{
        future::Future,
        pin::Pin,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };

    use next_web_core::{
        error::BoxError,
        scheduler::{
            repository::{InMemoryScheduledJobRepository, ScheduledJobReader, ScheduledJobStore},
            schedule_type::{ScheduleType, WithArgs},
            PersistedScheduledJob, ScheduledJobHandler, ScheduledJobRegistry,
        },
    };
    use serde_json::{json, Value};
    use tokio::sync::RwLock;

    use super::{BoxedJob, JobSchedulerManager};

    struct CountingHandler {
        task_key: &'static str,
        executions: Arc<AtomicUsize>,
        payloads: Arc<RwLock<Vec<Option<Value>>>>,
    }

    #[next_web_core::async_trait]
    impl ScheduledJobHandler for CountingHandler {
        fn task_key(&self) -> &'static str {
            self.task_key
        }

        async fn execute(
            &self,
            _context: next_web_core::scheduler::context::JobExecutionContext,
            payload: Option<Value>,
        ) -> Result<(), BoxError> {
            self.executions.fetch_add(1, Ordering::SeqCst);
            self.payloads.write().await.push(payload);
            Ok(())
        }
    }

    fn one_shot_schedule(seconds: u64) -> ScheduleType {
        ScheduleType::OneShot(WithArgs {
            initial_delay: Some(seconds),
            time_unit: Some("s".to_string()),
            ..Default::default()
        })
    }

    fn fixed_rate_schedule(seconds: u64) -> ScheduleType {
        ScheduleType::FixedRate(WithArgs {
            fixed_rate: Some(seconds),
            time_unit: Some("s".to_string()),
            ..Default::default()
        })
    }

    async fn wait_until<F>(timeout: Duration, mut predicate: F) -> bool
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = bool> + Send>>,
    {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if predicate().await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }

    #[tokio::test]
    async fn create_and_remove_persisted_job_syncs_repository_and_runtime_mapping() {
        let repository = Arc::new(InMemoryScheduledJobRepository::new());
        let registry = ScheduledJobRegistry::default();
        let manager =
            JobSchedulerManager::with_channel_size_and_repository(32, repository.clone(), registry)
                .await;

        let job =
            PersistedScheduledJob::new("job-1", "missing", one_shot_schedule(2)).enabled(false);
        manager
            .create_persisted(
                job.clone(),
                next_web_core::scheduler::context::JobExecutionContext::default(),
            )
            .await
            .unwrap();

        assert!(repository.find_by_id("job-1").await.unwrap().is_some());
        assert!(manager.exists_persisted("job-1").await.unwrap());

        let removed = manager.remove_persisted("job-1").await.unwrap();
        assert!(removed.is_some());
        assert!(repository.find_by_id("job-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn restore_only_enabled_jobs_and_skip_missing_handlers() {
        let repository = Arc::new(InMemoryScheduledJobRepository::new());
        let registry = ScheduledJobRegistry::default();
        let executions = Arc::new(AtomicUsize::new(0));
        let payloads = Arc::new(RwLock::new(Vec::new()));

        registry
            .register(Arc::new(CountingHandler {
                task_key: "handler.enabled",
                executions: executions.clone(),
                payloads: payloads.clone(),
            }))
            .unwrap();

        repository
            .save(
                PersistedScheduledJob::new("enabled", "handler.enabled", one_shot_schedule(1))
                    .with_payload(json!({"kind":"enabled"})),
            )
            .await
            .unwrap();
        repository
            .save(
                PersistedScheduledJob::new("disabled", "handler.enabled", one_shot_schedule(1))
                    .enabled(false),
            )
            .await
            .unwrap();
        repository
            .save(PersistedScheduledJob::new(
                "missing",
                "handler.missing",
                one_shot_schedule(1),
            ))
            .await
            .unwrap();

        let mut manager =
            JobSchedulerManager::with_channel_size_and_repository(32, repository.clone(), registry)
                .await;
        manager
            .restore_all(next_web_core::scheduler::context::JobExecutionContext::default())
            .await
            .unwrap();
        manager.start().await.unwrap();

        assert!(
            wait_until(Duration::from_secs(4), || {
                let executions = executions.clone();
                Box::pin(async move { executions.load(Ordering::SeqCst) == 1 })
            })
            .await
        );

        assert!(repository.find_by_id("enabled").await.unwrap().is_none());
        assert!(repository.find_by_id("disabled").await.unwrap().is_some());
        assert!(repository.find_by_id("missing").await.unwrap().is_some());
        assert_eq!(payloads.read().await.len(), 1);
    }

    #[tokio::test]
    async fn oneshot_job_is_deleted_and_last_run_at_is_updated_for_repeating_job() {
        let repository = Arc::new(InMemoryScheduledJobRepository::new());
        let registry = ScheduledJobRegistry::default();
        let executions = Arc::new(AtomicUsize::new(0));
        let payloads = Arc::new(RwLock::new(Vec::new()));

        registry
            .register(Arc::new(CountingHandler {
                task_key: "handler.run",
                executions: executions.clone(),
                payloads,
            }))
            .unwrap();

        let one_shot_job =
            PersistedScheduledJob::new("one-shot", "handler.run", one_shot_schedule(1));
        let repeating_job =
            PersistedScheduledJob::new("repeating", "handler.run", fixed_rate_schedule(1));

        let mut manager =
            JobSchedulerManager::with_channel_size_and_repository(32, repository.clone(), registry)
                .await;

        manager
            .create_persisted(
                one_shot_job,
                next_web_core::scheduler::context::JobExecutionContext::default(),
            )
            .await
            .unwrap();
        manager
            .create_persisted(
                repeating_job,
                next_web_core::scheduler::context::JobExecutionContext::default(),
            )
            .await
            .unwrap();
        manager.start().await.unwrap();

        assert!(
            wait_until(Duration::from_secs(4), || {
                let repository = repository.clone();
                Box::pin(async move {
                    repository
                        .find_by_id("repeating")
                        .await
                        .unwrap()
                        .and_then(|job| job.last_run_at)
                        .is_some()
                })
            })
            .await
        );

        assert!(
            wait_until(Duration::from_secs(4), || {
                let repository = repository.clone();
                Box::pin(async move { repository.find_by_id("one-shot").await.unwrap().is_none() })
            })
            .await
        );
    }

    #[tokio::test]
    async fn static_and_persisted_jobs_can_coexist_without_repository_pollution() {
        let repository = Arc::new(InMemoryScheduledJobRepository::new());
        let registry = ScheduledJobRegistry::default();
        let persisted_executions = Arc::new(AtomicUsize::new(0));
        let payloads = Arc::new(RwLock::new(Vec::new()));
        let static_executions = Arc::new(AtomicUsize::new(0));

        registry
            .register(Arc::new(CountingHandler {
                task_key: "handler.dual",
                executions: persisted_executions.clone(),
                payloads,
            }))
            .unwrap();

        let mut manager =
            JobSchedulerManager::with_channel_size_and_repository(32, repository.clone(), registry)
                .await;

        let static_counter = static_executions.clone();
        manager
            .add_static(BoxedJob::Sync((
                one_shot_schedule(1),
                Box::new(move || {
                    static_counter.fetch_add(1, Ordering::SeqCst);
                }),
            )))
            .await
            .unwrap();

        manager
            .create_persisted(
                PersistedScheduledJob::new("persisted", "handler.dual", one_shot_schedule(1))
                    .with_payload(json!({"source":"persisted"})),
                next_web_core::scheduler::context::JobExecutionContext::default(),
            )
            .await
            .unwrap();
        manager.start().await.unwrap();

        assert!(
            wait_until(Duration::from_secs(4), || {
                let static_executions = static_executions.clone();
                let persisted_executions = persisted_executions.clone();
                Box::pin(async move {
                    static_executions.load(Ordering::SeqCst) == 1
                        && persisted_executions.load(Ordering::SeqCst) == 1
                })
            })
            .await
        );

        assert_eq!(repository.count().await.unwrap(), 0);
    }
}

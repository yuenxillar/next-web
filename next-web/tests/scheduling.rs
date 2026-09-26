//! The scheduled tasks of an application, from the declaration to the state
//! the scheduler keeps of them.

#![cfg(feature = "enable-scheduling")]

use std::sync::{
    Arc,
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use next_web::{
    ConfigurableApplicationContext,
    autoconfigure::web_auto_configuration::WebAutoConfiguration,
    context::{ApplicationContextExt, DefaultApplicationContext},
    core::traits::config::auto_configuration::AutoConfiguration,
    macros::{bind::singleton, scheduled},
    scheduling::{
        InMemoryScheduledJobRepository, JobExecutionContext, JobSchedulerManager,
        PersistedScheduledJob, ScheduleType, ScheduledJobReader, ScheduledJobRegistry,
        ScheduledJobStore, ScheduledTask, WithArgs, JOB_SCHEDULER_MANAGER_SINGLETON_NAME,
    },
};
use next_web_core::async_trait;

/// Counts the runs of the task below.
static DECLARED_TASK_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Counts the runs of the provided task below.
static PROVIDED_TASK_RUNS: AtomicUsize = AtomicUsize::new(0);

/// Serializes the tests, which share the tasks of this binary.
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// A task an application declares by annotating a function.
#[scheduled(fixed_rate = 1, time_unit = "s", name = "test::declaredTask")]
async fn declared_task() {
    DECLARED_TASK_RUNS.fetch_add(1, Ordering::SeqCst);
}

/// A task an application provides itself.
#[singleton(default, binds = [Self::into_task])]
#[derive(Clone, Default)]
struct ProvidedTask;

impl ProvidedTask {
    fn into_task(self) -> Arc<dyn ScheduledTask> {
        Arc::new(self)
    }
}

#[async_trait]
impl ScheduledTask for ProvidedTask {
    fn schedule(&self) -> ScheduleType {
        ScheduleType::FixedRate(WithArgs {
            fixed_rate: Some(1),
            time_unit: Some(String::from("s")),
            ..Default::default()
        })
    }

    async fn execute(&self, _context: JobExecutionContext) {
        PROVIDED_TASK_RUNS.fetch_add(1, Ordering::SeqCst);
    }
}

/// Waits until `predicate` holds, or until the timeout passes.
async fn wait_until<F>(timeout: Duration, mut predicate: F) -> bool
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + timeout;

    while tokio::time::Instant::now() < deadline {
        if predicate() {
            return true;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    predicate()
}

/// Waits until the asynchronous `predicate` holds, or until the timeout passes.
async fn wait_until_async<F>(timeout: Duration, mut predicate: F) -> bool
where
    F: AsyncFnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + timeout;

    while tokio::time::Instant::now() < deadline {
        if predicate().await {
            return true;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    predicate().await
}

/// Starts an application context with a scheduler whose repository the test
/// keeps, so that the state of the tasks can be read.
async fn start_application(
    repository: Arc<InMemoryScheduledJobRepository>,
) -> DefaultApplicationContext {
    let mut context = DefaultApplicationContext::default();
    context.refresh().expect("the context refreshes");

    // The scheduler of the context is installed before the application is
    // configured, so that the test can read the schedules it keeps.
    let scheduler = JobSchedulerManager::with_channel_size_and_repository(
        32,
        repository,
        ScheduledJobRegistry::default(),
    )
    .await;
    context.insert_singleton_with_name(scheduler, JOB_SCHEDULER_MANAGER_SINGLETON_NAME);

    let mut auto_configuration = WebAutoConfiguration;
    auto_configuration
        .configure(&mut context)
        .await
        .expect("the application is configured");

    context
}

#[tokio::test(flavor = "multi_thread")]
async fn a_declared_task_runs_and_keeps_the_time_it_last_ran_at() {
    let _test = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let repository = Arc::new(InMemoryScheduledJobRepository::new());
    let _context = start_application(Arc::clone(&repository)).await;

    assert!(
        wait_until(Duration::from_secs(10), || {
            DECLARED_TASK_RUNS.load(Ordering::SeqCst) >= 2
        })
        .await,
        "the declared task should have run repeatedly"
    );

    // The time a task last ran at is written once the task returned, so it is
    // awaited separately from the run itself.
    assert!(
        wait_until_async(Duration::from_secs(10), || async {
            repository
                .find("test::declaredTask")
                .await
                .expect("the repository is readable")
                .and_then(|job| job.last_run_at)
                .is_some()
        })
        .await,
        "the time the task last ran at is recorded"
    );

    let job = repository
        .find("test::declaredTask")
        .await
        .expect("the repository is readable")
        .expect("the declared task is scheduled");

    assert!(job.enabled, "the declared task is enabled");
    assert_eq!(job.task_key, "test::declaredTask");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_provided_task_is_scheduled_under_the_name_of_its_provider() {
    let _test = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let repository = Arc::new(InMemoryScheduledJobRepository::new());
    let _context = start_application(Arc::clone(&repository)).await;

    assert!(
        wait_until(Duration::from_secs(10), || {
            PROVIDED_TASK_RUNS.load(Ordering::SeqCst) >= 2
        })
        .await,
        "the provided task should have run repeatedly (provided={}, declared={})",
        PROVIDED_TASK_RUNS.load(Ordering::SeqCst),
        DECLARED_TASK_RUNS.load(Ordering::SeqCst)
    );

    assert!(
        wait_until_async(Duration::from_secs(10), || async {
            repository
                .find("providedTask")
                .await
                .expect("the repository is readable")
                .and_then(|job| job.last_run_at)
                .is_some()
        })
        .await,
        "the time the task last ran at is recorded"
    );

    let job = repository
        .find("providedTask")
        .await
        .expect("the repository is readable")
        .expect("the provided task is scheduled");
    assert_eq!(job.task_key, "providedTask");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schedule_that_is_already_known_is_not_declared_again() {
    let _test = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let repository = Arc::new(InMemoryScheduledJobRepository::new());

    // A task an operator disabled before the application started is the state
    // the application has to keep: declaring the task again must not enable it.
    repository
        .save(
            PersistedScheduledJob::new(
                "test::declaredTask",
                "test::declaredTask",
                ScheduleType::FixedRate(WithArgs {
                    fixed_rate: Some(1),
                    time_unit: Some(String::from("s")),
                    ..Default::default()
                }),
            )
            .enabled(false),
        )
        .await
        .expect("the schedule is written");

    let runs_before = DECLARED_TASK_RUNS.load(Ordering::SeqCst);
    let _context = start_application(Arc::clone(&repository)).await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let stored = repository
        .find("test::declaredTask")
        .await
        .expect("the repository is readable")
        .expect("the declared task is still scheduled");

    assert!(!stored.enabled, "the disabled task stays disabled");
    assert_eq!(
        DECLARED_TASK_RUNS.load(Ordering::SeqCst),
        runs_before,
        "a task that is disabled does not run"
    );
}

//! The scheduling example.
//!
//! The example declares the tasks of an application in both of the ways the
//! framework supports:
//!
//! - a function is annotated with `#[scheduled]`, and the macro registers it
//!   with the scheduler, so the application never lists it,
//! - a task that implements `ScheduledTask` is provided as a singleton, which
//!   is the way to build a task whose schedule is computed.
//!
//! Every task is registered in the repository of the scheduler, which is what
//! keeps the time it last ran at. The repository is in memory here, so the state
//! lives as long as the application does; a repository backed by a database
//! makes the schedules and their state survive a restart.

use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering},
};

use next_web::{
    Application, NextWebApplication,
    core::async_trait,
    macros::{bind::singleton, scheduled},
    scheduling::{JobExecutionContext, ScheduleType, ScheduledTask, WithArgs},
    util::LocalDateTime,
};

/// The application of the example.
#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

/// Runs on a cron expression.
///
/// The expression has six fields, and the time zone it is evaluated in is the
/// one the attribute declares.
#[scheduled(cron = "*/3 * * * * *", timezone = "Asia/Shanghai")]
async fn cron_task() {
    println!("Cron Scheduled Task!       time: {}", LocalDateTime::now());
}

/// Runs on a fixed rate.
#[scheduled(fixed_rate = 2, time_unit = "s")]
async fn fixed_rate_task() {
    println!(
        "Fixed Rate Scheduled Task! time: {}",
        LocalDateTime::now()
    );
}

/// Counts the runs of the tasks of the example, and is resolved by the one shot
/// task below.
#[singleton(default)]
#[derive(Clone, Default)]
struct TaskCounter {
    value: Arc<AtomicI32>,
}

/// Runs once, after the declared delay.
///
/// The dependencies of a task are resolved from the application context, so a
/// task takes the instances it needs as its parameters.
#[scheduled(one_shot, initial_delay = 5, time_unit = "s")]
async fn one_shot_task(counter: TaskCounter) {
    counter.value.fetch_add(1, Ordering::Relaxed);

    println!(
        "One Shot Scheduled Task!   number: {}",
        counter.value.load(Ordering::Relaxed)
    );
}

/// A task that reports a failure.
///
/// The failure is logged by the scheduler, and the task keeps its schedule.
#[scheduled(fixed_rate = 3, time_unit = "s")]
async fn failing_task() -> Result<(), std::io::Error> {
    Err(std::io::Error::other("the task of the example failed"))
}

/// A task an application provides itself.
///
/// The name of the provider, which is the name of its type here, is the key of
/// the task: renaming it registers a new task instead of restoring this one.
#[singleton(default, binds = [Self::into_task])]
#[derive(Clone, Default)]
struct ProvidedTask {
    counter: Arc<AtomicI32>,
}

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
        self.counter.fetch_add(1, Ordering::Relaxed);

        println!(
            "Provided Scheduled Task!   number: {}",
            self.counter.load(Ordering::Relaxed)
        );
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

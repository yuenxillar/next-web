//! Scheduled tasks of an application.
//!
//! The module holds everything an application needs to run work on a schedule:
//!
//! - a task is declared with the `#[scheduled]` attribute on a function, or by
//!   providing a type that implements [`ScheduledTask`],
//! - the declaration is turned into a [`ScheduledJobRegistration`], which names
//!   the task, its [`ScheduleType`], and the [`ScheduledJobHandler`] that runs
//!   it,
//! - every task is kept in a [`PersistedScheduledJob`], which is the record of
//!   its schedule and of the time it last ran at, and which lives in a
//!   repository ([`ScheduledJobRepository`], an in-memory one by default),
//! - [`JobSchedulerManager`] drives the schedules and the repository,
//! - [`SchedulingBootstrap`] assembles the pieces while an application starts.
//!
//! # The life of a task
//!
//! 1. the macro, or the provider of a [`ScheduledTask`], contributes a
//!    registration to the application,
//! 2. [`SchedulingBootstrap`] registers the handler of the task under the key
//!    of the task, so that the task can be run by key alone,
//! 3. the bootstrap writes the declared schedule to the repository, and leaves
//!    the record alone when it already is there, so that the state of a task
//!    (whether it is enabled, when it last ran) is not overwritten while the
//!    application starts,
//! 4. the bootstrap schedules every enabled record, which includes the tasks an
//!    operator added to the repository while the application ran,
//! 5. the manager runs a task through its handler and records the time it last
//!    ran at.
//!
//! A one shot task that ran is removed from the repository, so it does not run
//! again when the application starts again.
//!
//! # What the state is worth
//!
//! The repository is the whole state of the scheduling of an application, and
//! what it survives is what survives a restart:
//!
//! - [`InMemoryScheduledJobRepository`] is what the framework uses by default.
//!   It keeps the schedules of the tasks an application declares, and the ones
//!   an application adds while it runs, but only for as long as the application
//!   does. It is the repository for a single process that does not have to
//!   remember anything.
//! - a repository an application installs under
//!   [`JOB_SCHEDULER_MANAGER_SINGLETON_NAME`] (before the application is
//!   configured) is used instead. An implementation of
//!   [`ScheduledJobRepository`] backed by a database or a cache makes the
//!   schedules, the states and the payloads of the tasks survive a restart: the
//!   tasks an application declares are written to it while the application
//!   starts, and everything it holds is scheduled again.
//!
//! A durable repository is also where the parts of the design that need an
//! application to agree with itself belong:
//!
//! - **Enabling a task**: a task that is disabled in the repository is not
//!   scheduled, and the declaration of the application does not enable it
//!   again, so an operator can turn a task off without removing the code.
//! - **Several instances**: every instance of an application restores the same
//!   schedules, so a durable repository has to decide which instance runs what.
//!   A repository can claim a task for one instance (with a lock, or by
//!   assigning tasks to instances), which is where a cluster belongs.
//! - **Missed runs**: a task whose time passed while an application was down is
//!   run when the schedule is restored. An application that has to catch up, or
//!   explicitly has to skip, decides it from the state the repository keeps.
//!
//! # Granularity
//!
//! The scheduler evaluates the schedules of an application once per second, so:
//!
//! - a cron expression resolves to the second,
//! - a fixed rate shorter than a second is raised to one second instead of
//!   being truncated to a schedule that never repeats, and a delay before the
//!   first run is raised the same way.

mod context;
mod persisted_job;
mod registry;
mod repository;
mod schedule_type;
mod scheduled_job_handler;
mod scheduled_job_reader;
mod scheduled_job_repository;
mod scheduled_job_store;
mod scheduled_task;

#[cfg(feature = "enable-scheduling")]
pub mod job_scheduler_manager;
pub mod scheduler_autoregister;
#[cfg(feature = "enable-scheduling")]
pub mod scheduling_bootstrap;

pub use context::JobExecutionContext;
pub use persisted_job::PersistedScheduledJob;
pub use registry::ScheduledJobRegistry;
pub use repository::InMemoryScheduledJobRepository;
pub use schedule_type::{ScheduleType, WithArgs};
pub use scheduler_autoregister::{ScheduledJobRegistration, SchedulerAutoRegister};
pub use scheduled_job_handler::ScheduledJobHandler;
pub use scheduled_job_reader::ScheduledJobReader;
pub use scheduled_job_repository::ScheduledJobRepository;
pub use scheduled_job_store::ScheduledJobStore;
pub use scheduled_task::ScheduledTask;

#[cfg(feature = "enable-scheduling")]
pub use job_scheduler_manager::{BoxedJob, JobSchedulerManager};
#[cfg(feature = "enable-scheduling")]
pub use scheduling_bootstrap::{
    JOB_SCHEDULER_MANAGER_SINGLETON_NAME, SCHEDULING_CHANNEL_SIZE_PROPERTY,
    SCHEDULING_ENABLED_PROPERTY, SchedulingBootstrap,
};

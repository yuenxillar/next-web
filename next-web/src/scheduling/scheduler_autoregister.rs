//! Registration of the scheduled tasks an application declares.
//!
//! A task is declared by annotating a function with the `#[scheduled]`
//! attribute. The macro submits a [`SchedulerAutoRegister`] to the inventory of
//! the application, which the scheduling bootstrap of the framework reads while
//! the application starts.
//!
//! Registering a task and scheduling it are two separate steps, which is what
//! makes the schedule of a task durable:
//!
//! 1. the registration declares the key, the schedule and the handler of the
//!    task,
//! 2. the bootstrap writes the schedule to the repository of the application
//!    and restores it from there, so a task the application declared in code
//!    and a task an operator added while the application ran are scheduled the
//!    same way, and both survive a restart when the repository does.

use std::sync::Arc;

use next_web_context::ApplicationContext;
use next_web_core::error::BoxError;
use serde_json::Value;

use super::{ScheduleType, ScheduledJobHandler};

/// A scheduled task an application declares.
///
/// The key identifies the task, both in the registry of the handlers and in the
/// repository of the schedules, so it is the one value that has to stay the
/// same across the restarts of an application: a task that is registered under
/// another key is scheduled again instead of being restored.
#[derive(Clone)]
pub struct ScheduledJobRegistration {
    task_key: String,
    schedule: ScheduleType,
    handler: Arc<dyn ScheduledJobHandler>,
    enabled: bool,
    payload: Option<Value>,
}

impl ScheduledJobRegistration {
    /// Creates the registration of a task.
    ///
    /// # Arguments
    ///
    /// * `task_key` - The key the task is registered under.
    /// * `schedule` - The schedule the task runs on.
    /// * `handler` - The handler that runs the task.
    pub fn new(
        task_key: impl Into<String>,
        schedule: ScheduleType,
        handler: Arc<dyn ScheduledJobHandler>,
    ) -> Self {
        Self {
            task_key: task_key.into(),
            schedule,
            handler,
            enabled: true,
            payload: None,
        }
    }

    /// Declares the task as disabled, which keeps it out of the schedule until
    /// it is enabled in the repository.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether the task runs.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Attaches the payload the handler receives when the task runs.
    ///
    /// # Arguments
    ///
    /// * `payload` - The payload of the task.
    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Returns the key the task is registered under.
    pub fn task_key(&self) -> &str {
        &self.task_key
    }

    /// Returns the schedule the task runs on.
    pub fn schedule(&self) -> &ScheduleType {
        &self.schedule
    }

    /// Returns the handler that runs the task.
    pub fn handler(&self) -> &Arc<dyn ScheduledJobHandler> {
        &self.handler
    }

    /// Returns whether the task runs.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the payload of the task, when it declares one.
    pub fn payload(&self) -> Option<&Value> {
        self.payload.as_ref()
    }
}

impl std::fmt::Debug for ScheduledJobRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScheduledJobRegistration")
            .field("task_key", &self.task_key)
            .field("schedule", &self.schedule)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

/// Contributes a scheduled task to the application.
///
/// The implementations are collected by the inventory and applied by the
/// scheduling bootstrap of the framework, which is why an application does not
/// have to list the functions it annotates with `#[scheduled]`.
///
/// The context is handed to the implementation because a task resolves its
/// dependencies from it: a task that takes an argument is given the instance
/// that is registered under the default name of the type of the argument.
pub trait SchedulerAutoRegister
where
    Self: Send + Sync + 'static,
{
    /// Resolves the task and returns its registration.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the dependencies of the task are resolved from.
    fn register(&self, ctx: &mut dyn ApplicationContext)
    -> Result<ScheduledJobRegistration, BoxError>;
}

inventory::collect!(&'static dyn SchedulerAutoRegister);

#[macro_export]
macro_rules! submit_scheduler {
    ($ty:ident) => {
        ::next_web::macros::submit! {
            &$ty as &dyn ::next_web::scheduling::SchedulerAutoRegister
        }
    };
}

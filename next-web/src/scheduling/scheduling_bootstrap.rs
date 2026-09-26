//! Starts the scheduler of an application.
//!
//! The bootstrap is what turns the `#[scheduled]` functions of an application
//! into running tasks. While the application starts it:
//!
//! 1. resolves the tasks an application declared, which the inventory of
//!    [`SchedulerAutoRegister`] contributed,
//! 2. registers the handler of every task in the registry of the scheduler, so
//!    that a task can be run by its key,
//! 3. writes the declared schedules to the repository of the application,
//! 4. restores every enabled schedule from that repository, which is where the
//!    tasks an operator added and the tasks of an earlier run come from,
//! 5. starts the scheduler.
//!
//! The repository decides how much of that state survives a restart. The
//! framework uses an in-memory repository by default, so schedules are restored
//! as long as the application runs; an application that installs a repository
//! backed by a database or a cache restores its schedules across restarts, with
//! the time each task last ran at.

use std::sync::Arc;

use next_web_context::{
    APPLICATION_ENVIRONMENT_SINGLETON_NAME, ApplicationContext, ApplicationContextExt,
};
use next_web_core::{async_trait, env::ConfigurableEnvironment, error::BoxError};
use tokio_cron_scheduler::JobSchedulerError;
use tracing::{debug, warn};

use super::{
    JobExecutionContext, JobSchedulerManager, ScheduledJobHandler, ScheduledJobRegistration,
    SchedulerAutoRegister, ScheduledTask,
};

/// The singleton the scheduler of an application is registered under.
pub const JOB_SCHEDULER_MANAGER_SINGLETON_NAME: &str = "jobSchedulerManager";

/// The property that turns the scheduling of an application off.
///
/// A value of `false` or `0` keeps every task from being scheduled, including
/// the tasks a repository holds.
pub const SCHEDULING_ENABLED_PROPERTY: &str = "next.scheduling.enabled";

/// The property that sets the size of the channel between the scheduler and the
/// tasks it runs.
pub const SCHEDULING_CHANNEL_SIZE_PROPERTY: &str = "next.scheduling.channel-size";

/// The number of tasks the scheduler buffers when the property is not set.
const DEFAULT_CHANNEL_SIZE: usize = 64;

/// Starts the scheduler of an application, and registers the tasks it declared.
///
/// The bootstrap runs once while the application starts, after the context is
/// refreshed, so that a task can resolve the singletons of the application.
pub struct SchedulingBootstrap;

impl SchedulingBootstrap {
    /// Registers the tasks an application declared and starts the scheduler.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the tasks resolve their dependencies from.
    ///
    /// # Errors
    ///
    /// Returns an error when a task cannot be resolved, when its schedule
    /// cannot be written to the repository of the application, or when the
    /// scheduler cannot be started.
    pub async fn configure(ctx: &mut dyn ApplicationContext) -> Result<(), BoxError> {
        if !Self::is_enabled(ctx) {
            debug!(
                property = SCHEDULING_ENABLED_PROPERTY,
                "the scheduling of the application is turned off"
            );
            return Ok(());
        }

        let mut manager = Self::manager(ctx).await;

        let registrations = Self::registrations(ctx)?;
        for registration in &registrations {
            // A task that is contributed twice, and a handler the application
            // registered itself, keep the registration that exists already; the
            // schedule of the task is written either way.
            if let Err(error) = manager.register_handler(Arc::clone(registration.handler())) {
                warn!(
                    task_key = registration.task_key(),
                    error = %error,
                    "the handler of a scheduled task is registered already"
                );
            }

            manager
                .declare_schedule(
                    registration.task_key(),
                    registration.schedule().clone(),
                    registration.is_enabled(),
                    registration.payload().cloned(),
                )
                .await?;
        }

        if !registrations.is_empty() {
            debug!(
                tasks = registrations.len(),
                "the scheduled tasks of the application are registered"
            );
        }

        // Restoring is what schedules every task, including the ones an
        // operator added to the repository while the application ran.
        manager
            .restore_all(JobExecutionContext::default())
            .await?;

        if let Err(error) = manager.start().await {
            // The scheduler of an application that was started already, which
            // happens when the bootstrap runs twice, is left as it is.
            if !matches!(error, JobSchedulerError::TickError) {
                return Err(Box::new(error));
            }

            debug!("the scheduler of the application is running already");
        }

        Ok(())
    }

    /// Returns whether the scheduling of the application is turned on.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context that holds the environment of the application.
    fn is_enabled(ctx: &dyn ApplicationContext) -> bool {
        let Some(environment) = ctx
            .get_singleton_option_with_name::<Arc<dyn ConfigurableEnvironment>>(
                APPLICATION_ENVIRONMENT_SINGLETON_NAME,
            )
        else {
            return true;
        };

        match environment.get_property(SCHEDULING_ENABLED_PROPERTY) {
            Some(value) => !matches!(value.trim().to_lowercase().as_str(), "false" | "0" | "off"),
            None => true,
        }
    }

    /// Returns the size of the channel between the scheduler and its tasks.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context that holds the environment of the application.
    fn channel_size(ctx: &dyn ApplicationContext) -> usize {
        ctx.get_singleton_option_with_name::<Arc<dyn ConfigurableEnvironment>>(
            APPLICATION_ENVIRONMENT_SINGLETON_NAME,
        )
        .and_then(|environment| environment.get_property(SCHEDULING_CHANNEL_SIZE_PROPERTY))
        .and_then(|size| size.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(DEFAULT_CHANNEL_SIZE)
    }

    /// Returns the manager of the application, and registers it when it does
    /// not exist yet.
    ///
    /// A manager an application installed itself is used, which is what lets an
    /// application keep its schedules in a repository of its own.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the manager is resolved from and registered in.
    async fn manager(ctx: &mut dyn ApplicationContext) -> JobSchedulerManager {
        if let Some(manager) = ctx
            .get_singleton_option_with_name::<JobSchedulerManager>(
                JOB_SCHEDULER_MANAGER_SINGLETON_NAME,
            )
            .cloned()
        {
            debug!("using the job scheduler manager of the application");
            return manager;
        }

        let manager = JobSchedulerManager::with_channel_size(Self::channel_size(ctx)).await;
        ctx.insert_singleton_with_name(
            manager.clone(),
            JOB_SCHEDULER_MANAGER_SINGLETON_NAME,
        );

        manager
    }

    /// Resolves the tasks an application declared.
    ///
    /// A task is declared in one of two ways:
    ///
    /// - by annotating a function with `#[scheduled]`, which submits a
    ///   [`SchedulerAutoRegister`] to the inventory,
    /// - by providing a task that implements [`ScheduledTask`], which is
    ///   registered under the name of its provider.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the dependencies of the tasks are resolved from.
    ///
    /// # Errors
    ///
    /// Returns the error of the first task that cannot be resolved, which keeps
    /// an application from starting with a task that never runs.
    fn registrations(ctx: &mut dyn ApplicationContext) -> Result<Vec<ScheduledJobRegistration>, BoxError>
    {
        let mut registrations = Vec::new();

        for register in inventory::iter::<&'static dyn SchedulerAutoRegister> {
            registrations.push(register.register(ctx)?);
        }

        registrations.extend(Self::provided_tasks(ctx));

        Ok(registrations)
    }

    /// Resolves the tasks an application provides as [`ScheduledTask`]
    /// instances.
    ///
    /// The name of the provider is the key of the task, so that naming a task
    /// provider is what keeps its schedule from being registered a second time
    /// after a restart.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the tasks are resolved from.
    fn provided_tasks(ctx: &mut dyn ApplicationContext) -> Vec<ScheduledJobRegistration> {
        let mut registrations = Vec::new();

        for key in ctx.keys_of_type(std::any::TypeId::of::<Arc<dyn ScheduledTask>>()) {
            let Some(instance) = ctx.resolve_boxed(&key) else {
                continue;
            };

            let Ok(task) = instance.downcast::<Arc<dyn ScheduledTask>>() else {
                continue;
            };

            let task_key = key.name.to_string();
            let schedule = task.schedule();

            debug!(
                task_key = task_key.as_str(),
                "a provided scheduled task is registered"
            );

            registrations.push(ScheduledJobRegistration::new(
                task_key.clone(),
                schedule,
                Arc::new(ProvidedTaskHandler {
                    task_key,
                    task: (*task).clone(),
                }),
            ));
        }

        registrations
    }
}

/// Runs a task an application provides as a [`ScheduledTask`].
///
/// The task is reached through the key it is provided under, which is what lets
/// the persisted scheduler run it after a restart, without the application
/// installing the task again.
struct ProvidedTaskHandler {
    task_key: String,
    task: Arc<dyn ScheduledTask>,
}

#[async_trait]
impl ScheduledJobHandler for ProvidedTaskHandler {
    fn id(&self) -> &str {
        &self.task_key
    }

    async fn execute(
        &self,
        context: JobExecutionContext,
        _payload: Option<serde_json::Value>,
    ) -> Result<(), BoxError> {
        self.task.execute(context).await;
        Ok(())
    }
}

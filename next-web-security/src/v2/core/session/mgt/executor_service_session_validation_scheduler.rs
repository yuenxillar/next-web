//! A session validation scheduler that runs on the async runtime.
//!
//! Apache Shiro's `ExecutorServiceSessionValidationScheduler` keeps a thread of
//! a pool and asks the session manager to sweep its store on an interval. The
//! port does the same with a task of the runtime the application already runs
//! on: enabling the scheduler spawns the task, disabling it aborts the task.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tracing::{debug, info};

use crate::core::session::mgt::{
    session_validation_scheduler::SessionValidationScheduler,
    validating_session_manager::ValidatingSessionManager,
};

/// A session validation scheduler backed by a task of the async runtime.
#[derive(Clone)]
pub struct ExecutorServiceSessionValidationScheduler {
    interval: Duration,
    session_manager: Arc<dyn ValidatingSessionManager>,
    task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ExecutorServiceSessionValidationScheduler {
    /// The interval a sweep runs on when none is asked for: an hour, the default
    /// of Shiro.
    pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(60 * 60);

    /// Creates a scheduler that sweeps the given manager on the default interval.
    ///
    /// # Arguments
    ///
    /// * `session_manager` - The manager whose store is swept.
    pub fn new(session_manager: Arc<dyn ValidatingSessionManager>) -> Self {
        Self::with_interval(session_manager, Self::DEFAULT_INTERVAL)
    }

    /// Creates a scheduler that sweeps the given manager on the given interval.
    ///
    /// # Arguments
    ///
    /// * `session_manager` - The manager whose store is swept.
    /// * `interval` - How often the sweep runs.
    pub fn with_interval(
        session_manager: Arc<dyn ValidatingSessionManager>,
        interval: Duration,
    ) -> Self {
        Self {
            interval,
            session_manager,
            task: Arc::new(Mutex::new(None)),
        }
    }

    /// Returns the interval the sweep runs on.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Returns the manager whose store is swept.
    pub fn session_manager(&self) -> &Arc<dyn ValidatingSessionManager> {
        &self.session_manager
    }
}

impl SessionValidationScheduler for ExecutorServiceSessionValidationScheduler {
    fn is_enabled(&self) -> bool {
        self.task
            .lock()
            .map(|task| task.is_some())
            .unwrap_or_default()
    }

    fn enable_session_validation(&self) {
        let Ok(mut task) = self.task.lock() else {
            return;
        };

        if task.is_some() {
            return;
        }

        let session_manager = self.session_manager.clone();
        let interval = self.interval;

        info!(
            "Enabling session validation scheduler, interval: {} ms",
            interval.as_millis()
        );

        *task = Some(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // The first tick of a `tokio` interval fires immediately; the sweep of
            // a fresh store has nothing to do, but skipping it keeps the first run
            // an interval away, the way a scheduled sweep is expected to behave.
            ticker.tick().await;

            loop {
                ticker.tick().await;
                debug!("Running session validation sweep.");

                session_manager.validate_sessions().await;
            }
        }));
    }

    fn disable_session_validation(&mut self) {
        let Ok(mut task) = self.task.lock() else {
            return;
        };

        if let Some(task) = task.take() {
            info!("Disabling session validation scheduler.");
            task.abort();
        }
    }
}


#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use next_web_core::async_trait;

    use super::ExecutorServiceSessionValidationScheduler;
    use crate::core::session::mgt::{
        session_validation_scheduler::SessionValidationScheduler,
        validating_session_manager::ValidatingSessionManager,
    };

    /// A manager that counts the sweeps it was asked for.
    #[derive(Default)]
    struct CountingSessionManager {
        sweeps: AtomicUsize,
        running: AtomicUsize,
    }

    #[async_trait]
    impl ValidatingSessionManager for CountingSessionManager {
        async fn validate_sessions(&self) {
            self.running.fetch_add(1, Ordering::SeqCst);
            self.sweeps.fetch_add(1, Ordering::SeqCst);
            self.running.fetch_sub(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn the_scheduler_sweeps_the_store_on_its_interval() {
        let manager = Arc::new(CountingSessionManager::default());
        let mut scheduler = ExecutorServiceSessionValidationScheduler::with_interval(
            manager.clone(),
            Duration::from_millis(10),
        );

        assert!(!scheduler.is_enabled());
        scheduler.enable_session_validation();
        assert!(scheduler.is_enabled());

        tokio::time::sleep(Duration::from_millis(80)).await;
        scheduler.disable_session_validation();

        assert!(!scheduler.is_enabled());
        assert!(
            manager.sweeps.load(Ordering::SeqCst) > 0,
            "the scheduler swept the store at least once"
        );
    }

    #[tokio::test]
    async fn a_disabled_scheduler_stops_sweeping() {
        let manager = Arc::new(CountingSessionManager::default());
        let mut scheduler = ExecutorServiceSessionValidationScheduler::with_interval(
            manager.clone(),
            Duration::from_millis(5),
        );

        scheduler.enable_session_validation();
        tokio::time::sleep(Duration::from_millis(40)).await;
        scheduler.disable_session_validation();
        let sweeps = manager.sweeps.load(Ordering::SeqCst);

        tokio::time::sleep(Duration::from_millis(40)).await;

        assert_eq!(
            manager.sweeps.load(Ordering::SeqCst),
            sweeps,
            "a disabled scheduler does not sweep any more"
        );
    }

    #[test]
    fn the_default_interval_is_an_hour() {
        assert_eq!(
            ExecutorServiceSessionValidationScheduler::DEFAULT_INTERVAL,
            Duration::from_secs(60 * 60)
        );
    }
}

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::http::{HttpRequest, HttpResponse};
use tracing::{info, trace};

use crate::core::session::{
    mgt::{
        default_native_session_manager::DefaultNativeSessionManager,
        native_session_manager::NativeSessionManagerExt,
        session_validation_scheduler::SessionValidationScheduler,
        validating_session::ValidatingSession,
    },
    Session, SessionError,
};

#[derive(Clone)]
pub struct DefaultValidatingSessionManager {
    // Fields and methods go here
    session_validation_scheduler_enabled: bool,
    session_validation_scheduler: Option<Box<dyn SessionValidationScheduler>>,
    session_validation_interval: i32,
    pub(crate) native_session_manager: DefaultNativeSessionManager,
}

impl DefaultValidatingSessionManager {
    const DEFAULT_SESSION_VALIDATION_INTERVAL: i32 = 3600000;

    pub fn is_session_validation_scheduler_enabled(&self) -> bool {
        self.session_validation_scheduler_enabled
    }

    pub fn set_session_validation_scheduler_enabled(&mut self, enabled: bool) {
        self.session_validation_scheduler_enabled = enabled;

        // Turning the scheduler on starts it, and turning it off stops it, which is
        // what Shiro's `setSessionValidationSchedulerEnabled` does through
        // `enableSessionValidation` and `disableSessionValidation`.
        if enabled {
            self.enable_session_validation();
        } else {
            self.disable_session_validation();
        }
    }

    pub fn set_session_validation_scheduler<T: SessionValidationScheduler + 'static>(
        &mut self,
        scheduler: T,
    ) {
        self.session_validation_scheduler = Some(Box::new(scheduler));

        // A scheduler that is handed to a manager that wants one is started at once,
        // the way Shiro's `setSessionValidationScheduler` leaves the enabling to the
        // initialization of the manager.
    }

    pub fn get_session_validation_scheduler(&self) -> Option<&dyn SessionValidationScheduler> {
        self.session_validation_scheduler.as_deref()
    }

    pub(crate) fn enable_session_validation_if_necessary(&self) {
        if let Some(scheduler) = self.get_session_validation_scheduler() {
            if self.is_session_validation_scheduler_enabled() && !scheduler.is_enabled() {
                self.enable_session_validation();
            }
        }
    }

    /// Stops the scheduler, when there is one that runs.
    ///
    /// Apache Shiro's `disableSessionValidation`: a manager that is told to stop
    /// sweeping stops the task it started, so a store is not swept behind the back
    /// of a deployment that turned the sweep off.
    pub fn disable_session_validation(&mut self) {
        let Some(scheduler) = self.session_validation_scheduler.as_mut() else {
            return;
        };

        if scheduler.is_enabled() {
            info!("Disabling session validation scheduler...");
            scheduler.disable_session_validation();
            self.after_session_validation_disabled();
        }
    }

    fn after_session_validation_disabled(&self) {}

    fn enable_session_validation(&self) {
        let scheduler = self.session_validation_scheduler.as_ref();

        if let Some(scheduler) = scheduler {
            if !scheduler.is_enabled() {
                info!("Enabling session validation scheduler...");

                scheduler.enable_session_validation();
                self.after_session_validation_enabled();
            }
        }
    }

    pub fn set_session_validation_interval(&mut self, session_validation_interval: i32) {
        self.session_validation_interval = session_validation_interval;
    }

    pub fn get_session_validation_interval(&self) -> i32 {
        self.session_validation_interval
    }

    pub async fn validate<T>(
        &self,
        session: &T,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), SessionError>
    where
        T: ValidatingSession + Session,
        T: NativeSessionManagerExt,
        T: Clone,
    {
        match session.validate() {
            Ok(_) => {}
            Err(ise) => match &ise {
                SessionError::Expired(_) => {
                    self.on_expiration(Arc::new(session.clone()), ise, session)
                        .await
                }
                SessionError::Invalid(_) => {
                    self._on_invalidation(Arc::new(session.clone()), ise, session, req, resp)
                        .await
                }
                _ => {}
            },
        };

        Ok(())
    }

    async fn _on_invalidation(
        &self,
        session: Arc<dyn Session>,
        ise: SessionError,
        ext: &dyn NativeSessionManagerExt,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        match &ise {
            SessionError::Expired(_msg) => {
                self.on_expiration(session, ise, ext).await;
                return;
            }
            _ => {}
        };

        trace!("Session with id [{}] is invalid.", session.id());

        ext.on_stop(&session, req, resp).await;
        self.native_session_manager.notify_stop(session.clone());
        ext.after_stopped(session.as_ref()).await;
    }

    async fn on_expiration(
        &self,
        session: Arc<dyn Session>,
        _e: SessionError,
        ext: &dyn NativeSessionManagerExt,
    ) {
        trace!("Session with id [{}] has expired.", session.id());

        ext.on_change(&session).await;
        self.native_session_manager
            .notify_expiration(session.clone());

        ext.after_stopped(session.as_ref()).await;
    }

    /// Called after session validation is enabled.
    fn after_session_validation_enabled(&self) {}
}

// `DefaultValidatingSessionManager` is the base of a session manager: the
// manager that holds the session store implements `ValidatingSessionManagerExt`,
// because expiring a session means removing it from that store.

// The sweep belongs to the manager that holds the session store, which is
// `DefaultSessionManager`; this base class only keeps the scheduler.

impl Deref for DefaultValidatingSessionManager {
    type Target = DefaultNativeSessionManager;

    fn deref(&self) -> &Self::Target {
        &self.native_session_manager
    }
}

impl DerefMut for DefaultValidatingSessionManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.native_session_manager
    }
}

impl Default for DefaultValidatingSessionManager {
    fn default() -> Self {
        Self {
            native_session_manager: Default::default(),
            session_validation_scheduler_enabled: true,
            session_validation_interval: Self::DEFAULT_SESSION_VALIDATION_INTERVAL,
            session_validation_scheduler: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use super::DefaultValidatingSessionManager;
    use crate::core::session::mgt::session_validation_scheduler::SessionValidationScheduler;

    /// A scheduler that records whether it was started.
    #[derive(Clone)]
    struct RecordingScheduler {
        enabled: Arc<AtomicBool>,
        started: Arc<AtomicBool>,
    }

    impl SessionValidationScheduler for RecordingScheduler {
        fn is_enabled(&self) -> bool {
            self.enabled.load(Ordering::SeqCst)
        }

        fn enable_session_validation(&self) {
            self.enabled.store(true, Ordering::SeqCst);
            self.started.store(true, Ordering::SeqCst);
        }

        fn disable_session_validation(&mut self) {
            self.enabled.store(false, Ordering::SeqCst);
        }
    }

    fn scheduler() -> (RecordingScheduler, Arc<AtomicBool>) {
        let enabled = Arc::new(AtomicBool::new(false));
        let scheduler = RecordingScheduler {
            enabled: enabled.clone(),
            started: Arc::new(AtomicBool::new(false)),
        };

        (scheduler, enabled)
    }

    #[test]
    fn turning_the_sweep_on_starts_the_scheduler_and_turning_it_off_stops_it() {
        let mut manager = DefaultValidatingSessionManager::default();
        let (scheduler, enabled) = scheduler();
        manager.set_session_validation_scheduler(scheduler);

        assert!(!enabled.load(Ordering::SeqCst), "a scheduler does not run before it is enabled");

        manager.set_session_validation_scheduler_enabled(true);
        assert!(enabled.load(Ordering::SeqCst), "the scheduler runs once the sweep is enabled");

        manager.set_session_validation_scheduler_enabled(false);
        assert!(!enabled.load(Ordering::SeqCst), "the scheduler stops once the sweep is disabled");
    }

    #[test]
    fn a_manager_without_a_scheduler_does_nothing() {
        let mut manager = DefaultValidatingSessionManager::default();

        assert!(manager.get_session_validation_scheduler().is_none());
        manager.set_session_validation_scheduler_enabled(true);
        manager.set_session_validation_scheduler_enabled(false);
        manager.enable_session_validation_if_necessary();
    }

    #[test]
    fn the_interval_of_the_sweep_is_readable() {
        let mut manager = DefaultValidatingSessionManager::default();
        let interval = manager.get_session_validation_interval();
        assert!(interval >= 0);
        manager.set_session_validation_interval(interval + 1);
        assert_eq!(manager.get_session_validation_interval(), interval + 1);
    }
}
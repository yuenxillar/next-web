use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::{error, info, trace};

use crate::core::session::{
    mgt::{
        default_native_session_manager::DefaultNativeSessionManager,
        native_session_manager::NativeSessionManagerExt,
        session_validation_scheduler::SessionValidationScheduler,
        validating_session::ValidatingSession,
        validating_session_manager::{ValidatingSessionManager, ValidatingSessionManagerExt},
    },
    Session, SessionError, SessionId,
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
    }

    pub fn set_session_validation_scheduler<T: SessionValidationScheduler + 'static>(
        &mut self,
        scheduler: T,
    ) {
        self.session_validation_scheduler = Some(Box::new(scheduler));
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

    /// Called after session validation is enabled. todo!
    fn after_session_validation_enabled(&self) {}
}

#[allow(unused_variables)]
#[async_trait]
impl ValidatingSessionManagerExt for DefaultValidatingSessionManager {
    async fn on_expiration(
        &self,
        session: &Arc<dyn Session>,
        error: SessionError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        todo!()
    }

    async fn get_active_sessions(&self) -> Vec<&Arc<dyn Session>> {
        todo!()
    }
}

impl ValidatingSessionManager for DefaultValidatingSessionManager {
    fn validate_sessions(&self) {}
}

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

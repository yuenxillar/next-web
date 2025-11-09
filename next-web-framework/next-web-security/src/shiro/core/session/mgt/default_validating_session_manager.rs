use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::{error, trace};

use crate::core::{
    authz::authorization_error::AuthorizationError,
    session::{
        expired_session_error::ExpiredSessionError,
        mgt::{
            default_native_session_manager::DefaultNativeSessionManager,
            native_session_manager::NativeSessionManagerExt,
            session_context::SessionContext,
            session_manager::SessionManager,
            session_validation_scheduler::SessionValidationScheduler,
            validating_session_manager::{ValidatingSessionManager, ValidatingSessionManagerExt},
        },
        Session, SessionError, SessionId,
    },
};

#[derive(Clone)]
pub struct DefaultValidatingSessionManager {
    // Fields and methods go here
    session_validation_scheduler_enabled: bool,
    session_validation_scheduler: Option<Arc<dyn SessionValidationScheduler>>,
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
        self.session_validation_scheduler = Some(Arc::new(scheduler));
    }

    pub fn get_session_validation_scheduler(&self) -> Option<&dyn SessionValidationScheduler> {
        self.session_validation_scheduler.as_deref()
    }

    fn enable_session_validation_if_necessary(&self) {
        if let Some(scheduler) = self.get_session_validation_scheduler() {
            if self.is_session_validation_scheduler_enabled() && !scheduler.is_enabled() {
                self.enable_session_validation();
            }
        }
    }

    pub fn set_session_validation_interval(&mut self, session_validation_interval: i32) {
        self.session_validation_interval = session_validation_interval;
    }

    pub fn get_session_validation_interval(&self) -> i32 {
        self.session_validation_interval
    }

    pub fn do_get_session(&self, session_id: &SessionId) {
        self.enable_session_validation_if_necessary();

        trace!("Attempting to retrieve session with key {}", session_id);
        match self.retrieve_session(session_id) {
            Some(session) => {
                self.validate(session, session_id);
            }
            None => {}
        };
        todo!()
    }

    pub fn retrieve_session(&self, session_id: &SessionId) -> Option<Box<dyn Session>> {
        todo!()
    }

    pub fn validate(&self, session: &dyn Session, session_id: &SessionId) {
        match session.validate() {
            Ok(_) => {}
            Err(err) => {
                error!("Session validation failed: {}", err);
                self.on_expiration(session, error, req, resp)
            }
        };
    }

    pub fn create_session(
        &self,
        context: &dyn SessionContext,
        ext: &dyn NativeSessionManagerExt,
    ) -> Result<Box<dyn Session>, AuthorizationError> {
        self.enable_session_validation_if_necessary();

        ext.do_get_session(session_id)
    }
}

#[async_trait]
impl ValidatingSessionManagerExt for DefaultValidatingSessionManager {
    async fn do_create_session(
        &self,
        ctx: &dyn SessionContext,
    ) -> Result<Arc<dyn Session>, AuthorizationError> {
        todo!()
    }

    async fn on_expiration(
        &self,
        session: &Arc<dyn Session>,
        error: ExpiredSessionError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
        todo!()
    }

    async fn after_expired(&self, session: &dyn Session) {
        todo!()
    }
}

impl ValidatingSessionManager for DefaultValidatingSessionManager {
    fn validate_sessions(&self) {}
}

impl SessionManager for DefaultValidatingSessionManager {
    fn start(&self, context: &dyn SessionContext) -> Result<Box<dyn Session>, AuthorizationError> {
        todo!()
    }

    fn get_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError> {
        todo!()
    }
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

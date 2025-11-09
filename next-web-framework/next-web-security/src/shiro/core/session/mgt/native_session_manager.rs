use std::{collections::HashSet, env::consts::ARCH, sync::Arc};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::{
    session::{
        mgt::{
            delegating_session::DelegatingSession, session_context::SessionContext,
            session_manager::SessionManager,
        },
        Session, SessionError, SessionId,
    },
    util::object::Object,
};

pub trait NativeSessionManager
where
    Self: Send + Sync,
    Self: SessionManager,
{
    fn start_time_stamp(&self, session_id: &SessionId) -> i64;

    fn last_access_time(&self, session_id: &SessionId) -> i64;

    fn is_valid(&self, session_id: &SessionId) -> bool;

    fn check_valid(&self, session_id: &SessionId) -> Result<(), SessionError>;

    fn timeout(&self, session_id: &SessionId) -> Result<i64, SessionError>;

    fn set_timeout(
        &mut self,
        session_id: &SessionId,
        max_idle_time_in_millis: i64,
    ) -> Result<(), SessionError>;

    fn touch(&self, session_id: &SessionId) -> Result<(), SessionError>;

    fn host(&self, session_id: &SessionId) -> Option<&str>;

    fn stop(&self, session_id: &SessionId) -> Result<(), SessionError>;

    fn attribute_keys(&self, session_id: &SessionId) -> Result<HashSet<String>, SessionError>;

    fn attribute(&self, session_id: &SessionId, key: &str) -> Result<Object, SessionError>;

    fn set_attribute(
        &self,
        session_id: &SessionId,
        key: &str,
        value: Object,
    ) -> Result<(), SessionError>;

    fn remove_attribute(&self, session_id: &SessionId, key: &str) -> Result<Object, SessionError>;
}

#[async_trait]
pub trait NativeSessionManagerExt
where
    Self: Send + Sync,
{
    #[allow(unused_variables)]
    async fn on_start(
        &self,
        session: &Arc<dyn Session>,
        ctx: &dyn SessionContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
    }

    async fn on_stop(
        &self,
        session: &Arc<dyn Session>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    );

    async fn after_stopped(&self, session: &mut dyn Session) {}

    async fn on_change(&self, session: &Arc<dyn Session>) {}

    fn create_exposed_session(&self, session: &dyn Session) -> Arc<dyn Session> {
        Arc::new(DelegatingSession::new(session.id().clone()))
    }

    fn do_get_session(
        &self,
        session_id: &SessionId,
        ctx: &dyn SessionContext,
    ) -> Result<Arc<dyn Session>, SessionError>;
}

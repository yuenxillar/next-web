use std::sync::Arc;

use next_web_core::{
    async_trait,
   http::{HttpRequest, HttpResponse},
};

use crate::core::session::{Session, SessionError};

/// A session manager that can sweep its store for expired sessions.
///
/// Apache Shiro's `AbstractValidatingSessionManager` keeps a scheduler that calls
/// the sweep on an interval; the sweep itself is what a deployment runs when it
/// does not want to wait for a request to notice that a session expired.
#[async_trait]
pub trait ValidatingSessionManager
where
    Self: Send + Sync,
{
    /// Drops every session of the store that is no longer valid.
    async fn validate_sessions(&self);
}

#[async_trait]
pub trait ValidatingSessionManagerExt
where
    Self: Send + Sync,
{
    // async fn do_create_session(
    //     &self,
    //     ctx: &dyn SessionContext,
    // ) -> Result<Arc<dyn Session>, AuthorizationError>;

    // async fn after_expired(&self, session: &dyn Session);

    async fn on_expiration(
        &self,
        session: &Arc<dyn Session>,
        error: SessionError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    );

    async fn on_invalidation(
        &self,
        _session: &dyn Session,
        _ise: SessionError,
        _req: &mut dyn HttpRequest,
        _resp: &mut dyn HttpResponse,
    ) {
    }

    async fn get_active_sessions(&self) -> Vec<Arc<dyn Session>>;
}

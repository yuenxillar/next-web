use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::{
    authz::authorization_error::AuthorizationError,
    session::{
        expired_session_error::ExpiredSessionError,
        mgt::{session_context::SessionContext, session_manager::SessionManager},
        Session, SessionError,
    },
};

pub trait ValidatingSessionManager
where
    Self: Send + Sync,
    Self: SessionManager,
{
    fn validate_sessions(&self);
}

#[async_trait]
pub trait ValidatingSessionManagerExt
where
    Self: Send + Sync,
{
    async fn do_create_session(
        &self,
        ctx: &dyn SessionContext,
    ) -> Result<Arc<dyn Session>, AuthorizationError>;

    async fn on_expiration(
        &self,
        session: &Arc<dyn Session>,
        error: ExpiredSessionError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    );

    async fn after_expired(&self, session: &dyn Session);

    async fn on_invalidation(
        &self,
        session: &dyn Session,
        ise: SessionError,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) {
    }
}

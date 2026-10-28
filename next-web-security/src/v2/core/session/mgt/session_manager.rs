use std::sync::Arc;

use next_web_core::{
    async_trait,
   http::{HttpRequest, HttpResponse},
};

use crate::core::{
    authz::authorization_error::AuthorizationError,
    session::{Session, SessionError, SessionId},
};

use super::session_context::SessionContext;

#[async_trait]
pub trait SessionManager
where
    Self: Send + Sync,
{
    async fn start(
        &self,
        context: &dyn SessionContext,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<Box<dyn Session>, AuthorizationError>;

    async fn get_session(&self, id: &SessionId) -> Result<Arc<dyn Session>, SessionError>;

    /// Creates a session for the given context.
    ///
    /// A subject that needs a session has no request at hand 鈥?it is the
    /// application that asks 鈥?so the manager creates the session from the
    /// context alone. A web manager that has to answer the request with a cookie
    /// reads the request in [`SessionManager::start`] instead.
    ///
    /// # Arguments
    ///
    /// * `context` - What the session is created with.
    ///
    /// # Errors
    ///
    /// Returns the error of a store that cannot create the session.
    async fn start_session(
        &self,
        context: &dyn SessionContext,
    ) -> Result<Arc<dyn Session>, SessionError>;
}

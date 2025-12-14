use std::sync::Arc;

use next_web_core::{async_trait, traits::http::{http_request::HttpRequest, http_response::HttpResponse}};

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
}

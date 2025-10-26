use std::sync::Arc;

use crate::core::{
    authz::authorization_error::AuthorizationError,
    session::{Session, SessionError, SessionId},
};

use super::session_context::SessionContext;

pub trait SessionManager
where
    Self: Send + Sync,
{
    fn start(&self, context: &dyn SessionContext) -> Result<Box<dyn Session>, AuthorizationError>;

    fn get_session(&self, id: & SessionId) -> Result<Arc<dyn Session>, SessionError>;
}

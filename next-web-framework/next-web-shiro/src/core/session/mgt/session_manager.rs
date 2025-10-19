use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::core::session::{Session, SessionId};

use super::session_context::SessionContext;

pub trait SessionManager
where
    Self: Send + Sync,
{
    fn start(&self, context: &dyn SessionContext) -> Arc<dyn Session>;

    fn get_session(&self, id: SessionId) -> Result<Arc<dyn Session>, BoxError>;
}

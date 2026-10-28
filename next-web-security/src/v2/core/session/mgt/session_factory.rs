use std::sync::Arc;

use crate::core::session::{mgt::session_context::SessionContext, Session};

pub trait SessionFactory
where
    Self: Send + Sync,
{
    fn create_session(&self, ctx: &dyn SessionContext) -> Arc<dyn Session>;
}

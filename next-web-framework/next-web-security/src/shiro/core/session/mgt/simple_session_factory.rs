use std::sync::Arc;

use crate::core::session::mgt::simple_session::SimpleSession;
use crate::core::session::{
    mgt::{session_context::SessionContext, session_factory::SessionFactory},
    Session,
};

#[derive(Clone)]
pub struct SimpleSessionFactory;

impl SessionFactory for SimpleSessionFactory {
    fn create_session(&self, ctx: &dyn SessionContext) -> Arc<dyn Session> {
        Arc::new(
            ctx.get_host()
                .map(|host| SimpleSession::new(host))
                .unwrap_or_default(),
        )
    }
}

impl Default for SimpleSessionFactory {
    fn default() -> Self {
        Self {}
    }
}

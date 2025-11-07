use next_web_core::convert::into_box::IntoBox;

use crate::core::session::mgt::simple_session::SimpleSession;
use crate::core::session::{
    mgt::{session_context::SessionContext, session_factory::SessionFactory},
    Session,
};

#[derive(Clone)]
pub struct SimpleSessionFactory;

impl SessionFactory for SimpleSessionFactory {
    fn create_session(&self, ctx: Option<&dyn SessionContext>) -> Box<dyn Session> {
        ctx.map(|context| {
            context
                .get_host()
                .map(|host| SimpleSession::new(host))
                .unwrap_or_default()
        })
        .unwrap_or_default()
        .into_boxed()
    }
}

impl Default for SimpleSessionFactory {
    fn default() -> Self {
        Self {}
    }
}

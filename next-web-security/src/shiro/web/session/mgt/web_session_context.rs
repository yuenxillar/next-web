
use crate::core::session::mgt::session_context::SessionContext;

pub trait WebSessionContext
where
    Self: Send +Sync,
    Self: SessionContext,
{}

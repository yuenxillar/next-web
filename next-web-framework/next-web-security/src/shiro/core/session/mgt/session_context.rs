use crate::core::{session::SessionId, util::object::Object};

pub trait SessionContext
where
    Self: Send +Sync
{
    fn set_host(&mut self, host: &str);

    fn get_host(&self) -> Option<&str>;

    fn set_session_id(&mut self, session_id: SessionId);

    fn get_session_id(&self) -> Option<&SessionId>;

    fn put_all(&mut self, values: Vec<(String, Object)>);
}

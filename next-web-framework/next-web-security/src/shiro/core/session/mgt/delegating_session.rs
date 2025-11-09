use crate::core::session::{Session, SessionError, SessionId, SessionValue};

#[derive(Clone)]
pub struct DelegatingSession {}

impl DelegatingSession {
    pub fn new(id: SessionId) -> Self {
        DelegatingSession {}
    }
}

impl Session for DelegatingSession {
    fn id(&self) -> &SessionId {
        todo!()
    }

    fn start_timestamp(&self) -> i64 {
        todo!()
    }

    fn last_access_time(&self) -> Option<i64> {
        todo!()
    }

    fn timeout(&self) -> Result<i64, SessionError> {
        todo!()
    }

    fn set_timeout(&mut self, timeout: i64) -> Result<(), SessionError> {
        todo!()
    }

    fn host(&self) -> Option<&str> {
        todo!()
    }

    fn touch(&self) -> Result<(), SessionError> {
        todo!()
    }

    fn stop(&self) -> Result<(), SessionError> {
        todo!()
    }

    fn attribute_keys(&self) -> Result<HashSet<String>, SessionError> {
        todo!()
    }

    fn get_attribute(&self, key: &str) -> Option<&SessionValue> {
        todo!()
    }

    fn set_attribute(&mut self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        todo!()
    }

    fn remove_attribute(&mut self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        todo!()
    }
}

use next_web_core::async_trait;

use crate::core::session::{Session, SessionError, SessionId, SessionValue};

#[derive(Clone)]
pub struct DelegatingSession {}

impl DelegatingSession {
    pub fn new(id: SessionId) -> Self {
        DelegatingSession {}
    }
}

#[async_trait]
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

    fn set_timeout(&self, timeout: i64) -> Result<(), SessionError> {
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

    async fn attribute_keys(&self) -> Result<Vec<String>, SessionError> {
        todo!()
    }

    async fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        todo!()
    }

    async fn set_attribute(&self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        todo!()
    }

    async fn remove_attribute(&self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        todo!()
    }
}

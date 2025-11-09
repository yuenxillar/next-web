use std::collections::HashSet;

use crate::core::session::{
    proxied_session::ProxiedSession, Session, SessionError, SessionId, SessionValue,
};

#[derive(Clone)]
pub struct ImmutableProxiedSession(pub(crate) ProxiedSession);

impl ImmutableProxiedSession {
    pub fn new(delegate: Box<dyn Session>) -> Self {
        Self(ProxiedSession::new(delegate))
    }

    pub fn error<T>(&self) -> Result<T, SessionError> {
        Err(SessionError::Invalid(
            "This session is immutable and read-only - it cannot be altered.
            This is usually because the session has been stopped or expired already."
                .to_string(),
        ))
    }
}

impl Session for ImmutableProxiedSession {
    fn id(&self) -> &SessionId {
        self.0.id()
    }

    fn start_timestamp(&self) -> i64 {
        self.0.start_timestamp()
    }

    fn last_access_time(&self) -> Option<i64> {
        self.0.last_access_time()
    }

    fn timeout(&self) -> Result<i64, SessionError> {
        self.0.timeout()
    }

    fn set_timeout(&mut self, _timeout: i64) -> Result<(), SessionError> {
        self.error()
    }

    fn host(&self) -> Option<&str> {
        self.0.host()
    }

    fn touch(&self) -> Result<(), SessionError> {
        self.error()
    }

    fn stop(&self) -> Result<(), SessionError> {
        self.error()
    }

    fn attribute_keys(&self) -> Result<HashSet<String>, SessionError> {
        self.0.attribute_keys()
    }

    fn get_attribute(&self, key: &str) -> Option<&SessionValue> {
        self.0.get_attribute(key)
    }

    fn set_attribute(&mut self, _key: &str, _value: SessionValue) -> Result<(), SessionError> {
        self.error()
    }

    fn remove_attribute(&mut self, _key: &str) -> Result<Option<SessionValue>, SessionError> {
        self.error()
    }
}

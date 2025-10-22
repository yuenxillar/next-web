use std::collections::HashSet;

use crate::core::session::{SessionId, SessionValue};

use super::Session;

#[derive(Clone)]
pub struct ProxiedSession {
    delegate: Box<dyn Session>,
}

impl ProxiedSession {
    pub fn new(delegate: Box<dyn Session>) -> Self {
        Self { delegate }
    }
}

impl Session for ProxiedSession {
    fn id(&self) -> SessionId {
        self.delegate.id()
    }

    fn start_timestamp(&self) -> std::time::SystemTime {
        self.delegate.start_timestamp()
    }

    fn last_access_time(&self) -> std::time::SystemTime {
        self.delegate.last_access_time()
    }

    fn timeout(&self) -> Result<u64, super::InvalidSessionError> {
        self.delegate.timeout()
    }

    fn set_timeout(
        &mut self,
        max_idle_time_in_millis: u64,
    ) -> Result<(), super::InvalidSessionError> {
        self.delegate.set_timeout(max_idle_time_in_millis)
    }

    fn host(&self) -> Option<&str> {
        self.delegate.host()
    }

    fn touch(&mut self) -> Result<(), super::InvalidSessionError> {
        self.delegate.touch()
    }

    fn stop(&mut self) -> Result<(), super::InvalidSessionError> {
        self.delegate.stop()
    }

    fn attribute_keys(&self) -> Result<HashSet<String>, super::InvalidSessionError> {
        self.delegate.attribute_keys()
    }

    fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        self.delegate.get_attribute(key)
    }

    fn set_attribute(
        &mut self,
        key: &str,
        value: SessionValue,
    ) -> Result<(), super::InvalidSessionError> {
        self.delegate.set_attribute(key, value)
    }

    fn remove_attribute(
        &mut self,
        key: &str,
    ) -> Result<Option<SessionValue>, super::InvalidSessionError> {
        self.delegate.remove_attribute(key)
    }
}

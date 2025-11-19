use std::sync::Arc;

use next_web_core::async_trait;

use super::{Session, SessionError};
use crate::core::session::{SessionId, SessionValue};

#[derive(Clone)]
pub struct ProxiedSession {
    delegate: Arc<dyn Session>,
}

impl ProxiedSession {
    pub fn new(delegate: Arc<dyn Session>) -> Self {
        Self { delegate }
    }
}

#[async_trait]
impl Session for ProxiedSession {
    fn id(&self) -> &SessionId {
        self.delegate.id()
    }

    fn start_timestamp(&self) -> i64 {
        self.delegate.start_timestamp()
    }

    fn last_access_time(&self) -> Option<i64> {
        self.delegate.last_access_time()
    }

    fn timeout(&self) -> Result<i64, SessionError> {
        self.delegate.timeout()
    }

    fn set_timeout(&self, timeout: i64) -> Result<(), SessionError> {
        self.delegate.set_timeout(timeout)
    }

    fn host(&self) -> Option<&str> {
        self.delegate.host()
    }

    fn touch(&self) -> Result<(), SessionError> {
        self.delegate.touch()
    }

    fn stop(&self) -> Result<(), SessionError> {
        self.delegate.stop()
    }

    async fn attribute_keys(&self) -> Result<Vec<String>, SessionError> {
        self.delegate.attribute_keys().await
    }

    async fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        self.delegate.get_attribute(key).await
    }

    async fn set_attribute(&self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        self.delegate.set_attribute(key, value).await
    }

    async fn remove_attribute(&self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        self.delegate.remove_attribute(key).await
    }
}

// impl ValidatingSession for ProxiedSession {
//     fn is_valid(&self) -> bool {
//         self.delegate.is_valid()
//     }

//     fn validate(&self) -> Result<(), BoxError> {
//         self.delegate.validate()
//     }
// }

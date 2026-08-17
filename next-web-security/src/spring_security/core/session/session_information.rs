use std::{fmt::Debug, time::SystemTime};

use crate::web::authentication::AuthPrincipal;

///
/// Represents a record of a session within the Next Security framework.
/// This is primarily used for concurrent session support.
#[derive(Clone)]
pub struct SessionInformation {
    last_request: SystemTime,
    principal: AuthPrincipal,
    session_id: String,
    expired: bool,
}

impl SessionInformation {
    pub fn new(
        principal: AuthPrincipal,
        session_id: impl Into<String>,
        last_request: SystemTime,
    ) -> Self {
        let session_id = session_id.into();
        assert!(!session_id.trim().is_empty(), "SessionId required");
        Self {
            last_request,
            principal,
            session_id,
            expired: false,
        }
    }

    pub fn expire_now(&mut self) {
        self.expired = true;
    }

    pub fn last_request(&self) -> SystemTime {
        self.last_request
    }

    pub fn principal(&self) -> &AuthPrincipal {
        &self.principal
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn is_expired(&self) -> bool {
        self.expired
    }

    /// Refreshes the internal lastRequest to the current date and time.
    pub fn refresh_last_request(&mut self) {
        self.last_request = SystemTime::now();
    }
}

impl Debug for SessionInformation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionInformation")
            .field("principal", &self.principal)
            .field("session_id", &self.session_id)
            .field("expired", &self.expired)
            .field("last_request", &self.last_request)
            .finish()
    }
}

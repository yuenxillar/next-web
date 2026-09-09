use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::oauth2::client::oidc_back_channel_logout::OidcLogoutToken;

/// The state persisted by an `OidcSessionRegistry` for a single provider session
/// that has been logged in via OIDC. Mirrors Spring Security's
/// `OidcSessionInformation`, which carries the session id, the authorities
/// granted to the session, and the `OidcUser` principal.
#[derive(Clone, Debug)]
pub struct OidcSessionInformation {
    session_id: String,
    authorities: HashMap<String, String>,
}

impl OidcSessionInformation {
    pub fn new(session_id: impl Into<String>, authorities: HashMap<String, String>) -> Self {
        Self {
            session_id: session_id.into(),
            authorities,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn authorities(&self) -> &HashMap<String, String> {
        &self.authorities
    }

    /// Returns a copy of this information with a new session id, mirroring
    /// `OidcSessionInformation#withSessionId`.
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }
}

/// Manages the link between an OIDC provider session (identified by the `sid`
////`sub` claims of a logout token) and the local application sessions that
/// should be terminated on back-channel logout.
pub trait OidcSessionRegistry: Send + Sync {
    /// Persists the given session information.
    fn save_session_information(&self, info: OidcSessionInformation);

    /// Removes and returns the session information identified by the given
    /// client session id.
    fn remove_session_information(&self, client_session_id: &str)
        -> Option<OidcSessionInformation>;

    /// Removes and returns every session information whose `sid` or `sub` matches
    /// the supplied logout token.
    fn remove_session_information_by_token(
        &self,
        logout_token: &OidcLogoutToken,
    ) -> Vec<OidcSessionInformation>;
}

/// The default, in-memory `OidcSessionRegistry`. Mirrors Spring Security's
/// `InMemoryOidcSessionRegistry`.
#[derive(Clone, Default)]
pub struct InMemoryOidcSessionRegistry {
    sessions: Arc<Mutex<HashMap<String, OidcSessionInformation>>>,
}

impl InMemoryOidcSessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OidcSessionRegistry for InMemoryOidcSessionRegistry {
    fn save_session_information(&self, info: OidcSessionInformation) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(info.session_id.clone(), info);
        }
    }

    fn remove_session_information(
        &self,
        client_session_id: &str,
    ) -> Option<OidcSessionInformation> {
        match self.sessions.lock() {
            Ok(mut sessions) => sessions.remove(client_session_id),
            Err(_) => None,
        }
    }

    fn remove_session_information_by_token(
        &self,
        logout_token: &OidcLogoutToken,
    ) -> Vec<OidcSessionInformation> {
        let mut removed = Vec::new();
        match self.sessions.lock() {
            Ok(mut sessions) => {
                let sid = logout_token.session_id();
                let sub = logout_token.subject();
                sessions.retain(|_id, info| {
                    let matches = info
                        .authorities
                        .get("sid")
                        .map(|s| Some(s.as_str()) == sid)
                        .unwrap_or(false)
                        || info
                            .authorities
                            .get("sub")
                            .map(|s| Some(s.as_str()) == sub)
                            .unwrap_or(false);
                    if matches {
                        removed.push(info.clone());
                        false
                    } else {
                        true
                    }
                });
            }
            Err(_) => {}
        }
        removed
    }
}

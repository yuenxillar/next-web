use next_web_core::async_trait;

use crate::web::authentication::AuthPrincipal;

use super::session_information::SessionInformation;

/// Maintains a registry of SessionInformation instances.
#[async_trait]
pub trait SessionRegistry
where
    Self: Send + Sync,
{
    /// Obtains all the known principals in the SessionRegistry.
    async fn all_principals(&self) -> Vec<String>;

    /// Obtains all the known sessions for the specified principal. Sessions that have been destroyed are not returned.
    /// Sessions that have expired may be returned, depending on the passed argument.
    async fn all_sessions(
        &self,
        principal: &AuthPrincipal,
        include_expired_sessions: bool,
    ) -> Vec<SessionInformation>;

    /// Obtains the session information for the specified sessionId. Even expired sessions are returned
    /// (although destroyed sessions are never returned).
    async fn session_information(&self, session_id: &str) -> Option<SessionInformation>;

    /// Updates the given sessionId so its last request time is equal to the present date and time.
    /// Silently returns if the given sessionId cannot be found or the session is marked to expire
    async fn refresh_last_request(&self, session_id: &str);

    /// Registers a new session for the specified principal.
    /// The newly registered session will not be marked for expiration.
    async fn register_new_session(&self, session_id: &str, principal: &AuthPrincipal);

    /// Deletes all the session information being maintained for the specified sessionId. If the
    /// sessionId is not found, the method gracefully returns.
    async fn remove_session_information(&self, session_id: &str);
}

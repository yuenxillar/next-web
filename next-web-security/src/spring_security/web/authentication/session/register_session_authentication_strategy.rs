use std::{fmt::Debug, sync::Arc};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::{session::SessionRegistry, Authentication, AuthenticationError},
    web::authentication::session::SessionAuthenticationStrategy,
};

/// Strategy used to register a user with the `SessionRegistry` after successful
/// `Authentication`.
///
/// `RegisterSessionAuthenticationStrategy` is typically used in combination with
/// `CompositeSessionAuthenticationStrategy` and
/// `ConcurrentSessionControlAuthenticationStrategy`, but can be used on its own if
/// tracking of sessions is desired but no need to control concurrency.
///
/// NOTE: When using a `SessionRegistry` it is important that all sessions (including
/// timed out sessions) are removed. This is typically done by adding
/// `HttpSessionEventPublisher`.
pub struct RegisterSessionAuthenticationStrategy {
    session_registry: Arc<dyn SessionRegistry>,
}

impl RegisterSessionAuthenticationStrategy {
    /// Creates a new instance.
    ///
    /// # Arguments
    ///
    /// * `session_registry` - The session registry which should be updated when the
    ///   authenticated session is changed. Cannot be null.
    pub fn new(session_registry: Arc<dyn SessionRegistry>) -> Self {
        Self { session_registry }
    }
}

#[async_trait]
impl SessionAuthenticationStrategy for RegisterSessionAuthenticationStrategy {
    async fn on_authentication(
        &self,
        authentication: &Arc<dyn Authentication>,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError> {
        let principal = match authentication.principal() {
            Some(principal) => principal,
            None => return Err(AuthenticationError::new("The principal cannot be none")),
        };

        let session_id = request.session_mut(true).map(|s| s.id());
        self.session_registry
            .register_new_session(session_id.unwrap_or_default(), principal.as_ref())
            .await;

        Ok(())
    }
}

impl Debug for RegisterSessionAuthenticationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisterSessionAuthenticationStrategy")
            .field("session_registry", &"none")
            .finish()
    }
}

use std::sync::Arc;

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::{enabled, Level};

use crate::{
    core::{authentication_error::AuthenticationError, Authentication},
    web::authentication::session::SessionAuthenticationStrategy,
};

/// A `SessionAuthenticationStrategy` that accepts multiple
/// `SessionAuthenticationStrategy` implementations to delegate to. Each
/// `SessionAuthenticationStrategy` is invoked in turn. The invocations are short
/// circuited if any exception, (i.e. `SessionAuthenticationException`) is thrown.
///
/// Typical usage would include having the following delegates (in this order):
///
/// * `ConcurrentSessionControlAuthenticationStrategy` - verifies that a user is allowed
///   to authenticate (i.e. they have not already logged into the application).
/// * `SessionFixationProtectionStrategy` - If session fixation is desired,
///   `SessionFixationProtectionStrategy` should be after
///   `ConcurrentSessionControlAuthenticationStrategy` to prevent unnecessary
///   `HttpSession` creation if the `ConcurrentSessionControlAuthenticationStrategy`
///   rejects authentication.
/// * `RegisterSessionAuthenticationStrategy` - It is important this is after
///   `SessionFixationProtectionStrategy` so that the correct session is registered.
pub struct CompositeSessionAuthenticationStrategy {
    delegate_strategies: Vec<Arc<dyn SessionAuthenticationStrategy>>,
}

impl CompositeSessionAuthenticationStrategy {
    /// Creates a new instance with the provided delegate strategies.
    ///
    /// # Arguments
    ///
    /// * `delegate_strategies` - The list of strategies to delegate to. Cannot be null
    ///   or empty, and must not contain null entries.
    ///
    /// # Panics
    ///
    /// Panics if `delegate_strategies` is empty or contains null entries.
    pub fn new(delegate_strategies: Vec<Arc<dyn SessionAuthenticationStrategy>>) -> Self {
        assert!(
            !delegate_strategies.is_empty(),
            "delegateStrategies cannot be null or empty"
        );

        Self {
            delegate_strategies,
        }
    }
}

#[async_trait]
impl SessionAuthenticationStrategy for CompositeSessionAuthenticationStrategy {
    async fn on_authentication(
        &self,
        authentication: &Arc<dyn Authentication>,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError> {
        let size = self.delegate_strategies.len();
        for (i, delegate) in self.delegate_strategies.iter().enumerate() {
            if enabled!(Level::TRACE) {
                tracing::trace!(
                    "Preparing session with Arc<dyn SessionAuthenticationStrategy> ({}/{})",
                    i + 1,
                    size
                );
            }
            delegate
                .on_authentication(authentication, req, resp)
                .await?;
        }
        Ok(())
    }
}

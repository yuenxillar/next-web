use std::{any::Any, sync::Arc};

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::core::Authentication;

/// Strategy for remember-me (persistent login) authentication.
/// Implementations read a remember-me token from the request (typically a cookie)
/// and return an `Authentication` if a valid, non-expired token is found.
#[async_trait]
pub trait RememberMeServices
where
    Self: Send + Sync,
    Self: Any,
{
    /// Attempt to auto-login the user from a remember-me token in the request.
    /// Returns `Some(Authentication)` on success, `None` if no token is present
    /// or the token is invalid/expired.
    async fn auto_login(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn Authentication>>;

    /// Called when interactive (e.g. form) login succeeds, so the
    /// implementation can create or refresh a remember-me token.
    async fn login_success(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: &dyn Authentication,
    );

    /// Called when interactive login fails, so the implementation can
    /// cancel any pending remember-me token.
    fn login_fail(&self, request: &mut dyn HttpRequest, response: &mut dyn HttpResponse);
}

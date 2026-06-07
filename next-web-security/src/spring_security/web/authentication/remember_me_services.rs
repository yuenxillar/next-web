use std::{any::Any, sync::Arc};

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::core::Authentication;

/// Strategy for remember-me (persistent login) authentication.
/// Implementations read a remember-me token from the request (typically a cookie)
/// and return an `Authentication` if a valid, non-expired token is found.
pub trait RememberMeServices
where
    Self: Send + Sync,
    Self: Any,
{
    /// Attempt to auto-login the user from a remember-me token in the request.
    /// Returns `Some(Authentication)` on success, `None` if no token is present
    /// or the token is invalid/expired.
    fn auto_login(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Option<Arc<dyn Authentication>>;

    /// Called when interactive (e.g. form) login succeeds, so the
    /// implementation can create or refresh a remember-me token.
    fn login_success(
        &self,
        _request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _authentication: &dyn Authentication,
    ) {
    }

    /// Called when interactive login fails, so the implementation can
    /// cancel any pending remember-me token.
    fn login_fail(
        &self,
        _request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
    }
}

use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    core::{context::security_context_holder::SecurityContextHolder, Authentication},
    web::authentication::logout::LogoutHandler,
};

/// A `LogoutHandler` that clears the `SecurityContext` from the
/// `SecurityContextHolder`.
///
/// This is the default logout handler used by `ConcurrentSessionFilter`.
#[derive(Clone, Default)]
pub struct SecurityContextLogoutHandler;

#[async_trait]
impl LogoutHandler for SecurityContextLogoutHandler {
    async fn logout(
        &self,
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _authentication: Option<&dyn Authentication>,
    ) {
        SecurityContextHolder::clear_context();
    }
}

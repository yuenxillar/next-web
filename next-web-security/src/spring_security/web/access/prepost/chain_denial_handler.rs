//! The refusal of a rule is rendered the way the filter chain renders its own
//! refusals.

use std::sync::Arc;

use next_web_core::{
    error::BoxError,
    security::PreAuthorizeDenialHandler,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    access::AccessDeniedError, authorization::AuthorizationDeniedError,
    web::access::AccessDeniedHandler,
};

/// Renders the refusal of a rule with the access denied handler of the
/// application.
///
/// The handler is the one the application configured for the filter chain, so a
/// handler that is refused by its own rule is answered the way the filter chain
/// answers a refused request.
#[derive(Clone)]
pub struct ChainDenialHandler {
    access_denied_handler: Arc<dyn AccessDeniedHandler>,
}

impl ChainDenialHandler {
    /// Creates the renderer of a refusal.
    ///
    /// # Arguments
    ///
    /// * `access_denied_handler` - The handler of the application.
    pub fn new(access_denied_handler: Arc<dyn AccessDeniedHandler>) -> Self {
        Self {
            access_denied_handler,
        }
    }
}

impl PreAuthorizeDenialHandler for ChainDenialHandler {
    fn render(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        self.access_denied_handler.handle(
            request,
            response,
            &AccessDeniedError::AuthorizationDenied(AuthorizationDeniedError::with_message(
                "Access Denied",
            )),
        )
    }
}

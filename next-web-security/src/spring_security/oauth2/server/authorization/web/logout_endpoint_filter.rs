//! The endpoint that logs a resource owner out of the authorization server.

use next_web_core::{
    async_trait,
    filter::FilterError,
    http::{HttpMethod, StatusCode},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use serde_json::json;

use super::{
    super::{client_authentication::form_parameter, context::AuthorizationServerContext},
    support::{json_response, send_redirect},
};
use crate::core::context::SecurityContextHolder;

/// Serves the logout endpoint of the authorization server.
///
/// The endpoint is asked by the client, or by the resource owner, when the
/// session of the server ends. The security context of the session is cleared,
/// the session is invalidated, and the resource owner is sent back to the URI
/// the request names when that URI is one the server knows.
#[derive(Clone)]
pub struct LogoutEndpointFilter {
    context: AuthorizationServerContext,
}

impl LogoutEndpointFilter {
    /// Creates the filter of the endpoint.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self { context }
    }

    /// Returns the URI the resource owner is sent back to, if the server knows
    /// it.
    ///
    /// The URI has to be the redirect URI of one of the registered clients, so
    /// a request cannot send the resource owner to another site.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    fn post_logout_redirect_uri(&self, request: &dyn HttpRequest) -> Option<String> {
        let candidate = form_parameter(request, "post_logout_redirect_uri")?;
        let known = self
            .context
            .registered_clients()
            .list()
            .into_iter()
            .any(|client| client.has_redirect_uri(&candidate));

        known.then_some(candidate)
    }
}

#[async_trait]
impl HttpFilter for LogoutEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().end_session_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        if request.method() != HttpMethod::GET && request.method() != HttpMethod::POST {
            json_response(
                response,
                StatusCode::METHOD_NOT_ALLOWED,
                &json!({ "error": "invalid_request" }),
            );
            return Ok(());
        }

        // The session of the resource owner ends, so its security context is
        // cleared and the session itself is invalidated.
        SecurityContextHolder::get_context_holder_strategy().clear_context();
        if let Some(session) = request.session_mut(false) {
            session.invalidate();
        }

        match self.post_logout_redirect_uri(request) {
            Some(uri) => send_redirect(request, response, &uri, &[]),
            None => {
                json_response(
                    response,
                    StatusCode::OK,
                    &json!({ "message": "The resource owner is logged out" }),
                );
                Ok(())
            }
        }
    }
}

impl Named for LogoutEndpointFilter {
    fn name(&self) -> &str {
        "LogoutEndpointFilter"
    }
}

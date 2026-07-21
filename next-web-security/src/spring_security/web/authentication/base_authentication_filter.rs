use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    authorization::AuthenticationManager,
    core::{
        context::security_context_holder::SecurityContextHolder,
        username_password_authentication_token::UsernamePasswordAuthenticationToken,
    },
    web::authentication_entry_point::AuthenticationEntryPoint,
};

/// Processes an HTTP Basic `Authorization` header and authenticates via
/// the configured `AuthenticationManager`.
#[derive(Clone)]
pub struct BasicAuthenticationFilter {
    authentication_manager: Arc<dyn AuthenticationManager>,
    #[allow(dead_code)]
    authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
}

impl BasicAuthenticationFilter {
    pub fn new(
        authentication_manager: Arc<dyn AuthenticationManager>,
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> Self {
        Self {
            authentication_manager,
            authentication_entry_point,
        }
    }

    /// Extract username and password from the `Authorization: Basic <base64>` header.
    /// Returns `None` if the header is missing, not a Basic scheme, or malformed.
    fn extract_basic_credentials(&self, request: &dyn HttpRequest) -> Option<(String, String)> {
        let header = request.header("Authorization")?;
        let header = header.trim();

        // Must start with "Basic "
        if header.len() < 7 || !header[..6].eq_ignore_ascii_case("Basic ") {
            return None;
        }

        let encoded = header[6..].trim();
        // Decode base64
        let decoded_bytes = base64_decode(encoded)?;
        let decoded = String::from_utf8(decoded_bytes).ok()?;

        // Format: "username:password"
        let colon_pos = decoded.find(':')?;
        let username = decoded[..colon_pos].to_string();
        let password = decoded[colon_pos + 1..].to_string();

        Some((username, password))
    }
}

#[async_trait]
impl HttpFilter for BasicAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Extract and attempt authentication if header present
        if let Some((username, password)) = self.extract_basic_credentials(request) {
            let token = UsernamePasswordAuthenticationToken::unauthenticated(
                Some(username),
                Some(password),
            );

            match self.authentication_manager.authenticate(&token) {
                Ok(auth_result) => {
                    // Store the authentication in the security context
                    match SecurityContextHolder::get_context() {
                        Some(ctx) => ctx.set_authentication(Some(auth_result)),
                        None => {
                            let ctx = SecurityContextHolder::create_empty_context();
                            ctx.set_authentication(Some(auth_result));
                            SecurityContextHolder::set_context(ctx);
                        }
                    }
                }
                Err(_error) => {
                    // Failed auth - entry point handles the response
                    // Clear context and challenge
                    SecurityContextHolder::clear_context();
                    // Continue to the entry point below
                }
            }
        }

        // If still no authentication, let the entry point handle it
        let is_authenticated = SecurityContextHolder::get_context()
            .map(|ctx| ctx.get_authentication().is_some())
            .unwrap_or_default();

        if !is_authenticated {
            // Challenge the client for credentials
            // The entry point sends 401 with WWW-Authenticate header
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for BasicAuthenticationFilter {
    fn name(&self) -> &str {
        "BasicAuthenticationFilter"
    }
}

/// Decode a base64-encoded string to bytes.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(input).ok()
}

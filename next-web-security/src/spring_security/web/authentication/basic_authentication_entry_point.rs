use next_web_core::http::StatusCode;
use next_web_core::traits::http::http_response::HttpResponse;
use next_web_core::{error::BoxError, traits::http::http_request::HttpRequest};

use crate::{
    core::authentication_error::AuthenticationError,
    web::authentication_entry_point::AuthenticationEntryPoint,
};

/// Sends a `401 Unauthorized` response with a `WWW-Authenticate: Basic`
/// header, challenging the client to provide HTTP Basic credentials.
#[derive(Clone)]
pub struct BasicAuthenticationEntryPoint {
    realm_name: String,
}

impl BasicAuthenticationEntryPoint {
    /// Create a new entry point with the given realm name.
    pub fn new(realm_name: &str) -> Self {
        Self {
            realm_name: realm_name.to_string(),
        }
    }

    pub fn set_realm_name(&mut self, realm_name: &str) {
        self.realm_name = realm_name.to_string();
    }
}

impl AuthenticationEntryPoint for BasicAuthenticationEntryPoint {
    fn commence(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _auth_error: Option<AuthenticationError>,
    ) -> Result<(), BoxError> {
        let header_value = format!(r#"Basic realm="{}""#, self.realm_name);
        response.set_status_code(StatusCode::UNAUTHORIZED);
        response.insert_header("WWW-Authenticate", &header_value);
        response.set_body(Default::default());

        Ok(())
    }
}

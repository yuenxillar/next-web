use axum::{
    extract::Request,
    http::StatusCode,
    response::Response,
};
use next_web_core::error::BoxError;

use crate::{core::authentication_error::AuthenticationError, web::authentication_entry_point::AuthenticationEntryPoint};

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
        _request: &mut Request,
        _response: &mut Response,
        _auth_error: Option<AuthenticationError>,
    ) -> Result<(), BoxError> {
        let header_value = format!(r#"Basic realm="{}""#, self.realm_name);
        let resp = Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", &header_value)
            .body(axum::body::Body::empty())
            .map_err(|e| Box::new(e) as BoxError)?;
        *_response = resp;
        Ok(())
    }
}

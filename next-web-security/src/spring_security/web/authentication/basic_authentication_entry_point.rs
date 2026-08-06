use next_web_core::http::StatusCode;
use next_web_core::traits::http::http_response::HttpResponse;
use next_web_core::util::StringUtils;
use next_web_core::{error::BoxError, traits::http::http_request::HttpRequest};

use crate::{
    core::authentication_error::AuthenticationError,
    web::authentication_entry_point::AuthenticationEntryPoint,
};

/// Used by the ExceptionTranslationFilter to commence authentication via the BasicAuthenticationFilter.
/// Once a user agent is authenticated using BASIC authentication, logout requires that the browser be closed or an unauthorized (401) header be sent.
/// The simplest way of achieving the latter is to call the commence(HttpServletRequest, HttpServletResponse, AuthenticationException) method below.
/// This will indicate to the browser its credentials are no longer authorized, causing it to prompt the user to login again.
#[derive(Clone)]
pub struct BasicAuthenticationEntryPoint {
    realm_name: Option<String>,
    header_value: Option<String>,
}

impl BasicAuthenticationEntryPoint {
    /// Create a new entry point with the given realm name.
    pub fn new(realm_name: impl Into<String>) -> Self {
        let mut entry_point = Self {
            realm_name: Some(realm_name.into()),
            header_value: None,
        };

        entry_point.update_header_value();
        entry_point
    }

    pub fn set_realm_name(&mut self, realm_name: impl Into<String>) {
        self.realm_name = Some(realm_name.into());
        self.update_header_value();
    }

    pub fn get_realm_name(&self) -> Option<&str> {
        self.realm_name.as_deref()
    }

    pub fn after_properties_set(&self) {
        assert!(
            self.realm_name
                .as_deref()
                .map(StringUtils::has_text)
                .unwrap_or_default(),
            "realm_name must be specified"
        );
    }

    fn update_header_value(&mut self) {
        if self.header_value.is_none() {
            let header_value = format!(
                r#"Basic realm="{}""#,
                self.realm_name
                    .as_deref()
                    .expect("realm_name must be specified")
            );
            self.header_value = Some(header_value);
        }
    }
}

impl AuthenticationEntryPoint for BasicAuthenticationEntryPoint {
    fn commence(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _auth_error: &AuthenticationError,
    ) -> Result<(), BoxError> {
        let header_value = self
            .header_value
            .as_deref()
            .expect("header_value must be specified");
        response.set_status_code(StatusCode::UNAUTHORIZED);
        response.insert_header("WWW-Authenticate", header_value);
        response.set_body(b"Unauthorized".to_vec());

        Ok(())
    }
}

impl Default for BasicAuthenticationEntryPoint {
    fn default() -> Self {
        Self {
            realm_name: None,
            header_value: None,
        }
    }
}

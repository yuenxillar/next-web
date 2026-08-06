use crate::{
    authorization::AuthenticationDetailsSource,
    core::{
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
        Authentication, UsernamePasswordAuthenticationToken,
    },
    web::authentication::{AuthenticationConverter, WebAuthenticationDetailsSource},
};
use base64::Engine;
use next_web_core::{traits::http::http_request::HttpRequest, util::StringUtils};
use std::sync::Arc;

/// Converts from a `HttpServletRequest` to `UsernamePasswordAuthenticationToken` that
/// can be authenticated. Returns `None` if there was no Authorization header with Basic
/// authentication scheme.
#[derive(Clone)]
pub struct BasicAuthenticationConverter {
    authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
}

impl BasicAuthenticationConverter {
    /// The Basic authentication scheme prefix.
    pub const AUTHENTICATION_SCHEME_BASIC: &'static str = "Basic";

    /// Creates a new instance with the specified `AuthenticationDetailsSource`.
    ///
    /// # Arguments
    ///
    /// * `authentication_details_source` - The source for building authentication
    ///   details.
    pub fn with_authentication_details_source(
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) -> Self {
        Self {
            authentication_details_source,
        }
    }

    /// Returns the authentication details source.
    pub fn get_authentication_details_source(&self) -> &Arc<dyn AuthenticationDetailsSource> {
        &self.authentication_details_source
    }

    /// Sets the authentication details source.
    ///
    /// # Arguments
    ///
    /// * `authentication_details_source` - The source for building authentication
    ///   details. Must not be null.
    pub fn set_authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = authentication_details_source;
    }

    /// Decodes a Base64-encoded byte array.
    ///
    /// # Arguments
    ///
    /// * `base64_token` - The Base64-encoded bytes to decode.
    ///
    /// # Errors
    ///
    /// Returns `BadCredentialsException` if decoding fails.
    fn decode(&self, base64_token: &[u8]) -> Result<Vec<u8>, AuthenticationError> {
        use base64::engine::general_purpose::STANDARD;
        STANDARD.decode(base64_token).map_err(|_| {
            AuthenticationError::with_kind(
                "Failed to decode basic authentication token",
                AuthenticationErrorKind::BadCredentials,
            )
        })
    }
}

impl AuthenticationConverter for BasicAuthenticationConverter {
    /// Converts a `HttpServletRequest` to a `UsernamePasswordAuthenticationToken`.
    ///
    /// Returns `None` if there is no Authorization header or if the header does not use
    /// the Basic authentication scheme.
    ///
    /// # Arguments
    ///
    /// * `request` - The HTTP request to extract credentials from.
    ///
    /// # Errors
    ///
    /// Returns `BadCredentialsException` if the Basic authentication token is empty or
    /// malformed, or if Base64 decoding fails.
    fn convert(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        // Extract the Authorization header.
        let header = match request.header("authorization").map(str::trim) {
            Some(header) => header,
            None => return Ok(None),
        };

        // Check if the header starts with "Basic" (case-insensitive).
        if !StringUtils::starts_with_ignore_case(header, Self::AUTHENTICATION_SCHEME_BASIC) {
            return Ok(None);
        }

        // Reject empty Basic token (header equals exactly "Basic").
        if header.eq_ignore_ascii_case(Self::AUTHENTICATION_SCHEME_BASIC) {
            return Err(AuthenticationError::with_kind(
                "Empty basic authentication token",
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        // Extract the Base64-encoded portion after "Basic ".
        // "Basic " is 6 characters, so we skip those 6 bytes.
        let base64_token = &header[6..];
        let base64_bytes = base64_token.as_bytes();

        // Decode the Base64 token.
        let decoded = self.decode(base64_bytes)?;
        let token = String::from_utf8(decoded).map_err(|_| {
            AuthenticationError::with_kind(
                "Invalid basic authentication token encoding",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        // Split on the first colon to separate username and password.
        let delim = token.find(':').ok_or_else(|| {
            AuthenticationError::with_kind(
                "Invalid basic authentication token",
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        let username = token[..delim].to_string();
        let password = token[delim + 1..].to_string();

        // Create an unauthenticated token.
        let mut result = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(Arc::new(username)),
            Some(Arc::new(password)),
        );

        // Set the authentication details.
        result.set_details(Some(
            self.authentication_details_source.build_details(request),
        ));

        Ok(Some(Box::new(result)))
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Default for BasicAuthenticationConverter {
    fn default() -> Self {
        Self::with_authentication_details_source(
            Arc::new(WebAuthenticationDetailsSource::default()),
        )
    }
}

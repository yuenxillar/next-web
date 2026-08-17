use crate::core::{AuthenticationError, AuthenticationErrorKind};

use super::digest_auth_utils;

/// Holds the parsed Digest HTTP Authorization header data.
///
/// Handles both RFC 2069 and RFC 2617 (with qop=auth) digest formats.
pub struct DigestData {
    username: Option<String>,
    realm: Option<String>,
    nonce: Option<String>,
    uri: Option<String>,
    response: Option<String>,
    qop: Option<String>,
    nc: Option<String>,
    cnonce: Option<String>,
    /// The raw section 2.1.2 response string (everything after "Digest ").
    section_212response: String,
    /// The expiry time parsed from the nonce in milliseconds.
    nonce_expiry_time: i64,
}

impl DigestData {
    /// Parses the "Authorization" header value into a `DigestData`.
    ///
    /// # Parameters
    /// * `header` - The full Authorization header value starting with "Digest ".
    pub fn new(header: &str) -> Self {
        let section_212response = header[7..].to_string();
        let header_entries = digest_auth_utils::split_ignoring_quotes(&section_212response, ',');
        let header_map =
            digest_auth_utils::split_each_array_element_and_create_map(&header_entries, "=", "\"");

        let username = header_map.get("username").cloned();
        let realm = header_map.get("realm").cloned();
        let nonce = header_map.get("nonce").cloned();
        let uri = header_map.get("uri").cloned();
        let response = header_map.get("response").cloned();
        let qop = header_map.get("qop").cloned();
        let nc = header_map.get("nc").cloned();
        let cnonce = header_map.get("cnonce").cloned();

        Self {
            username,
            realm,
            nonce,
            uri,
            response,
            qop,
            nc,
            cnonce,
            section_212response,
            nonce_expiry_time: 0,
        }
    }

    /// Validates the parsed digest data and decodes the nonce.
    ///
    /// Checks that all required fields are present, the realm matches,
    /// and that the nonce is properly formed and signed.
    ///
    /// # Parameters
    /// * `entry_point_key` - The secret key used to sign the nonce.
    /// * `expected_realm`  - The expected realm name.
    pub fn validate_and_decode(
        &mut self,
        entry_point_key: &str,
        expected_realm: &str,
    ) -> Result<(), AuthenticationError> {
        // Check all required parameters were supplied (RFC 2069).
        if self.username.is_none()
            || self.realm.is_none()
            || self.nonce.is_none()
            || self.uri.is_none()
            || self.response.is_none()
        {
            return Err(AuthenticationError::with_kind(
                format!(
                    "Missing mandatory digest value; received header {}",
                    self.section_212response
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        // Check all required parameters for "auth" qop were supplied (RFC 2617).
        if self.qop.as_deref() == Some("auth") {
            if self.nc.is_none() || self.cnonce.is_none() {
                return Err(AuthenticationError::with_kind(
                    format!(
                        "Missing mandatory digest value; received header {}",
                        self.section_212response
                    ),
                    AuthenticationErrorKind::BadCredentials,
                ));
            }
        }

        // Check realm name equals what we expected.
        let realm = self.realm.as_deref().unwrap_or("");
        if realm != expected_realm {
            return Err(AuthenticationError::with_kind(
                format!(
                    "Response realm name '{}' does not match system realm name of '{}'",
                    realm, expected_realm
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        // Decode nonce from Base64.
        let nonce_str = self.nonce.as_deref().unwrap_or("");
        let nonce_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, nonce_str).map_err(
                |_| {
                    AuthenticationError::with_kind(
                        format!(
                            "Nonce is not encoded in Base64; received nonce {}",
                            nonce_str
                        ),
                        AuthenticationErrorKind::BadCredentials,
                    )
                },
            )?;

        let nonce_as_plain_text = String::from_utf8_lossy(&nonce_bytes);
        let nonce_tokens: Vec<&str> = nonce_as_plain_text.split(':').collect();

        if nonce_tokens.len() != 2 {
            return Err(AuthenticationError::with_kind(
                format!(
                    "Nonce should have yielded two tokens but was {}",
                    nonce_as_plain_text
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        // Extract expiry time from nonce.
        self.nonce_expiry_time = nonce_tokens[0].parse::<i64>().map_err(|_| {
            AuthenticationError::with_kind(
                format!(
                    "Nonce token should have yielded a numeric first token, but was {}",
                    nonce_as_plain_text
                ),
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        // Check signature of nonce matches this expiry time.
        let expected_nonce_signature =
            digest_auth_utils::md5_hex(&format!("{}:{}", self.nonce_expiry_time, entry_point_key));
        if expected_nonce_signature != nonce_tokens[1] {
            return Err(AuthenticationError::with_kind(
                format!("Nonce token compromised {}", nonce_as_plain_text),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        Ok(())
    }

    /// Calculates the expected server-side digest for comparison with the
    /// client-supplied response.
    pub fn calculate_server_digest(
        &self,
        password: Option<&str>,
        http_method: &str,
        password_already_encoded: bool,
    ) -> String {
        digest_auth_utils::generate_digest(
            password_already_encoded,
            self.username.as_deref().unwrap_or(""),
            self.realm.as_deref().unwrap_or(""),
            password,
            http_method,
            self.uri.as_deref().unwrap_or(""),
            self.qop.as_deref(),
            self.nonce.as_deref().unwrap_or(""),
            self.nc.as_deref().unwrap_or(""),
            self.cnonce.as_deref().unwrap_or(""),
        )
    }

    /// Returns whether the nonce has expired.
    pub fn is_nonce_expired(&self, now_millis: i64) -> bool {
        self.nonce_expiry_time < now_millis
    }

    /// Returns the parsed username, if present.
    pub fn get_username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Returns the client's digest response.
    pub fn get_response(&self) -> Option<&str> {
        self.response.as_deref()
    }
}

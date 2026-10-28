//! The settings of the authorization server.

use serde_json::{json, Value};

/// The paths and the values the endpoints of the authorization server are
/// served with.
///
/// The endpoint paths are the ones of the specifications by default, and the
/// issuer is the value the metadata of the server and the issuer claim of the
/// tokens are built with. A deployment sets the issuer, because the value is
/// part of the identity of the server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationServerSettings {
    issuer: Option<String>,
    authorization_endpoint: String,
    token_endpoint: String,
    jwk_set_endpoint: String,
    revocation_endpoint: String,
    introspection_endpoint: String,
    authorization_server_metadata_endpoint: String,
    openid_configuration_endpoint: String,
    user_info_endpoint: String,
    end_session_endpoint: String,
    pushed_authorization_request_endpoint: String,
    device_authorization_endpoint: String,
    device_verification_endpoint: String,
    client_registration_endpoint: String,
    client_registration_initial_access_token: Option<String>,
    allow_open_client_registration: bool,
    require_pushed_authorization_requests: bool,
    trust_forwarded_client_certificate: bool,
    scopes: Vec<String>,
}

impl AuthorizationServerSettings {
    /// The path of the authorization endpoint of the specification.
    pub const DEFAULT_AUTHORIZATION_ENDPOINT: &'static str = "/oauth2/authorize";

    /// The path of the token endpoint of the specification.
    pub const DEFAULT_TOKEN_ENDPOINT: &'static str = "/oauth2/token";

    /// The path of the JWK set endpoint of the specification.
    pub const DEFAULT_JWK_SET_ENDPOINT: &'static str = "/oauth2/jwks";

    /// The path of the token revocation endpoint of the specification.
    pub const DEFAULT_REVOCATION_ENDPOINT: &'static str = "/oauth2/revoke";

    /// The path of the token introspection endpoint of the specification.
    pub const DEFAULT_INTROSPECTION_ENDPOINT: &'static str = "/oauth2/introspect";

    /// The path of the authorization server metadata endpoint of the
    /// specification.
    pub const DEFAULT_AUTHORIZATION_SERVER_METADATA_ENDPOINT: &'static str =
        "/.well-known/oauth-authorization-server";

    /// The path of the discovery document of the OpenID Connect provider.
    pub const DEFAULT_OPENID_CONFIGURATION_ENDPOINT: &'static str =
        "/.well-known/openid-configuration";

    /// The path of the user info endpoint.
    pub const DEFAULT_USER_INFO_ENDPOINT: &'static str = "/userinfo";

    /// The path of the logout endpoint of the resource owner.
    pub const DEFAULT_END_SESSION_ENDPOINT: &'static str = "/connect/logout";

    /// The path of the pushed authorization request endpoint.
    pub const DEFAULT_PUSHED_AUTHORIZATION_REQUEST_ENDPOINT: &'static str = "/oauth2/par";

    /// The path of the device authorization endpoint.
    pub const DEFAULT_DEVICE_AUTHORIZATION_ENDPOINT: &'static str = "/oauth2/device_authorization";

    /// The path of the page the user code of a device is entered on.
    pub const DEFAULT_DEVICE_VERIFICATION_ENDPOINT: &'static str = "/oauth2/device_verification";

    /// The path of the client registration endpoint.
    pub const DEFAULT_CLIENT_REGISTRATION_ENDPOINT: &'static str = "/connect/register";

    /// Creates a builder for the settings.
    pub fn builder() -> AuthorizationServerSettingsBuilder {
        AuthorizationServerSettingsBuilder::default()
    }

    /// Returns the issuer of the authorization server, if one is set.
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    /// Returns the issuer, or the given value when no issuer is set.
    ///
    /// # Arguments
    ///
    /// * `default_issuer` - The issuer to use when the settings do not name one.
    pub fn issuer_or(&self, default_issuer: &str) -> String {
        self.issuer
            .clone()
            .unwrap_or_else(|| default_issuer.to_string())
    }

    /// Returns the path of the authorization endpoint.
    pub fn authorization_endpoint(&self) -> &str {
        &self.authorization_endpoint
    }

    /// Returns the path of the token endpoint.
    pub fn token_endpoint(&self) -> &str {
        &self.token_endpoint
    }

    /// Returns the path of the JWK set endpoint.
    pub fn jwk_set_endpoint(&self) -> &str {
        &self.jwk_set_endpoint
    }

    /// Returns the path of the token revocation endpoint.
    pub fn revocation_endpoint(&self) -> &str {
        &self.revocation_endpoint
    }

    /// Returns the path of the token introspection endpoint.
    pub fn introspection_endpoint(&self) -> &str {
        &self.introspection_endpoint
    }

    /// Returns the path of the authorization server metadata endpoint.
    pub fn authorization_server_metadata_endpoint(&self) -> &str {
        &self.authorization_server_metadata_endpoint
    }

    /// Returns the path of the discovery document of the OpenID Connect
    /// provider.
    pub fn openid_configuration_endpoint(&self) -> &str {
        &self.openid_configuration_endpoint
    }

    /// Returns the path of the user info endpoint.
    pub fn user_info_endpoint(&self) -> &str {
        &self.user_info_endpoint
    }

    /// Returns the path of the logout endpoint of the resource owner.
    pub fn end_session_endpoint(&self) -> &str {
        &self.end_session_endpoint
    }

    /// Returns the path of the pushed authorization request endpoint.
    pub fn pushed_authorization_request_endpoint(&self) -> &str {
        &self.pushed_authorization_request_endpoint
    }

    /// Returns the path of the device authorization endpoint.
    pub fn device_authorization_endpoint(&self) -> &str {
        &self.device_authorization_endpoint
    }

    /// Returns the path of the page the user code of a device is entered on.
    pub fn device_verification_endpoint(&self) -> &str {
        &self.device_verification_endpoint
    }

    /// Returns the path of the client registration endpoint.
    pub fn client_registration_endpoint(&self) -> &str {
        &self.client_registration_endpoint
    }

    /// Returns the token a client presents to register itself, if one is
    /// configured.
    ///
    /// A request to the client registration endpoint is served only when the
    /// endpoint is open or when the request presents this token as a bearer
    /// token.
    pub fn client_registration_initial_access_token(&self) -> Option<&str> {
        self.client_registration_initial_access_token.as_deref()
    }

    /// Returns whether any client may register itself.
    ///
    /// Open registration is disabled by default, because an endpoint that any
    /// client may register itself with is an endpoint any client may use to
    /// obtain credentials.
    pub fn allow_open_client_registration(&self) -> bool {
        self.allow_open_client_registration
    }

    /// Returns whether an authorization request has to be pushed.
    ///
    /// When the value is `true`, the authorization endpoint serves only a
    /// request that names the request URI of a request that was pushed to the
    /// pushed authorization request endpoint.
    pub fn require_pushed_authorization_requests(&self) -> bool {
        self.require_pushed_authorization_requests
    }

    /// Returns whether the certificate of a client may be read from a header
    /// that the terminator of the TLS connection forwarded.
    ///
    /// The value decides whether a client may authenticate with
    /// `tls_client_auth` or `self_signed_tls_client_auth` while the application
    /// itself does not terminate the TLS connection. It is disabled by default,
    /// because a header that carries a certificate is trusted only when the
    /// application can trust the terminator that set it.
    pub fn trust_forwarded_client_certificate(&self) -> bool {
        self.trust_forwarded_client_certificate
    }

    /// Returns the scopes the server advertises, if any are configured.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Returns the metadata document of the authorization server.
    ///
    /// The document is the `oauth-authorization-server` metadata of the
    /// specification, so a client can discover the endpoints of the server.
    pub fn metadata(&self) -> Value {
        self.server_metadata()
    }

    /// Returns the discovery document of the OpenID Connect provider.
    ///
    /// The document is the metadata of the server with the values an OpenID
    /// Connect client reads: the user info endpoint, the logout endpoint, and
    /// the algorithms the identifier tokens are signed with.
    pub fn openid_configuration(&self) -> Value {
        let issuer = self.issuer_or("");
        let mut metadata = self.server_metadata();
        metadata["userinfo_endpoint"] = json!(format!("{issuer}{}", self.user_info_endpoint));
        metadata["end_session_endpoint"] = json!(format!("{issuer}{}", self.end_session_endpoint));
        metadata["subject_types_supported"] = json!(["public"]);
        metadata["id_token_signing_alg_values_supported"] =
            json!([crate::oauth2::jose::RS256, crate::oauth2::jose::HS256]);
        metadata["device_authorization_endpoint"] =
            json!(format!("{issuer}{}", self.device_authorization_endpoint));
        metadata["registration_endpoint"] =
            json!(format!("{issuer}{}", self.client_registration_endpoint));

        metadata
    }

    /// Returns the metadata the two discovery documents are built from.
    fn server_metadata(&self) -> Value {
        let issuer = self.issuer_or("");
        let mut metadata = json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{issuer}{}", self.authorization_endpoint),
            "token_endpoint": format!("{issuer}{}", self.token_endpoint),
            "jwks_uri": format!("{issuer}{}", self.jwk_set_endpoint),
            "revocation_endpoint": format!("{issuer}{}", self.revocation_endpoint),
            "introspection_endpoint": format!("{issuer}{}", self.introspection_endpoint),
            "pushed_authorization_request_endpoint":
                format!("{issuer}{}", self.pushed_authorization_request_endpoint),
            "require_pushed_authorization_requests": self.require_pushed_authorization_requests,
            "response_types_supported": ["code"],
            "grant_types_supported": [
                "authorization_code",
                "client_credentials",
                "refresh_token",
                "urn:ietf:params:oauth:grant-type:device_code",
            ],
            "token_endpoint_auth_methods_supported": [
                "client_secret_basic",
                "client_secret_post",
                "private_key_jwt",
                "tls_client_auth",
                "self_signed_tls_client_auth",
                "none",
            ],
            "revocation_endpoint_auth_methods_supported": [
                "client_secret_basic",
                "client_secret_post",
            ],
            "introspection_endpoint_auth_methods_supported": [
                "client_secret_basic",
                "client_secret_post",
            ],
            "code_challenge_methods_supported": ["S256", "plain"],
        });

        if !self.scopes.is_empty() {
            metadata["scopes_supported"] = json!(self.scopes);
        }

        metadata
    }
}

impl Default for AuthorizationServerSettings {
    fn default() -> Self {
        Self {
            issuer: None,
            authorization_endpoint: Self::DEFAULT_AUTHORIZATION_ENDPOINT.to_string(),
            token_endpoint: Self::DEFAULT_TOKEN_ENDPOINT.to_string(),
            jwk_set_endpoint: Self::DEFAULT_JWK_SET_ENDPOINT.to_string(),
            revocation_endpoint: Self::DEFAULT_REVOCATION_ENDPOINT.to_string(),
            introspection_endpoint: Self::DEFAULT_INTROSPECTION_ENDPOINT.to_string(),
            authorization_server_metadata_endpoint:
                Self::DEFAULT_AUTHORIZATION_SERVER_METADATA_ENDPOINT.to_string(),
            openid_configuration_endpoint: Self::DEFAULT_OPENID_CONFIGURATION_ENDPOINT.to_string(),
            user_info_endpoint: Self::DEFAULT_USER_INFO_ENDPOINT.to_string(),
            end_session_endpoint: Self::DEFAULT_END_SESSION_ENDPOINT.to_string(),
            pushed_authorization_request_endpoint:
                Self::DEFAULT_PUSHED_AUTHORIZATION_REQUEST_ENDPOINT.to_string(),
            device_authorization_endpoint: Self::DEFAULT_DEVICE_AUTHORIZATION_ENDPOINT.to_string(),
            device_verification_endpoint: Self::DEFAULT_DEVICE_VERIFICATION_ENDPOINT.to_string(),
            client_registration_endpoint: Self::DEFAULT_CLIENT_REGISTRATION_ENDPOINT.to_string(),
            client_registration_initial_access_token: None,
            allow_open_client_registration: false,
            require_pushed_authorization_requests: false,
            trust_forwarded_client_certificate: false,
            scopes: Vec::new(),
        }
    }
}

/// Builds the settings of the authorization server.
#[derive(Clone, Debug, Default)]
pub struct AuthorizationServerSettingsBuilder {
    settings: AuthorizationServerSettings,
}

impl AuthorizationServerSettingsBuilder {
    /// Sets the issuer of the authorization server.
    ///
    /// # Arguments
    ///
    /// * `issuer` - The identifier of the server, without a trailing slash.
    pub fn issuer(mut self, issuer: &str) -> Self {
        assert!(!issuer.trim().is_empty(), "issuer cannot be empty");
        self.settings.issuer = Some(issuer.trim_end_matches('/').to_string());
        self
    }

    /// Sets the path of the authorization endpoint.
    ///
    /// # Arguments
    ///
    /// * `authorization_endpoint` - The path of the endpoint.
    pub fn authorization_endpoint(mut self, authorization_endpoint: &str) -> Self {
        self.settings.authorization_endpoint = authorization_endpoint.to_string();
        self
    }

    /// Sets the path of the token endpoint.
    ///
    /// # Arguments
    ///
    /// * `token_endpoint` - The path of the endpoint.
    pub fn token_endpoint(mut self, token_endpoint: &str) -> Self {
        self.settings.token_endpoint = token_endpoint.to_string();
        self
    }

    /// Sets the path of the JWK set endpoint.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_endpoint` - The path of the endpoint.
    pub fn jwk_set_endpoint(mut self, jwk_set_endpoint: &str) -> Self {
        self.settings.jwk_set_endpoint = jwk_set_endpoint.to_string();
        self
    }

    /// Sets the path of the token revocation endpoint.
    ///
    /// # Arguments
    ///
    /// * `revocation_endpoint` - The path of the endpoint.
    pub fn revocation_endpoint(mut self, revocation_endpoint: &str) -> Self {
        self.settings.revocation_endpoint = revocation_endpoint.to_string();
        self
    }

    /// Sets the path of the token introspection endpoint.
    ///
    /// # Arguments
    ///
    /// * `introspection_endpoint` - The path of the endpoint.
    pub fn introspection_endpoint(mut self, introspection_endpoint: &str) -> Self {
        self.settings.introspection_endpoint = introspection_endpoint.to_string();
        self
    }

    /// Sets the scopes the server advertises.
    ///
    /// # Arguments
    ///
    /// * `scopes` - The scopes of the server.
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.settings.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the path of the pushed authorization request endpoint.
    ///
    /// # Arguments
    ///
    /// * `pushed_authorization_request_endpoint` - The path of the endpoint.
    pub fn pushed_authorization_request_endpoint(
        mut self,
        pushed_authorization_request_endpoint: &str,
    ) -> Self {
        self.settings.pushed_authorization_request_endpoint =
            pushed_authorization_request_endpoint.to_string();
        self
    }

    /// Sets the path of the device authorization endpoint.
    ///
    /// # Arguments
    ///
    /// * `device_authorization_endpoint` - The path of the endpoint.
    pub fn device_authorization_endpoint(mut self, device_authorization_endpoint: &str) -> Self {
        self.settings.device_authorization_endpoint = device_authorization_endpoint.to_string();
        self
    }

    /// Sets the path of the page the user code of a device is entered on.
    ///
    /// # Arguments
    ///
    /// * `device_verification_endpoint` - The path of the page.
    pub fn device_verification_endpoint(mut self, device_verification_endpoint: &str) -> Self {
        self.settings.device_verification_endpoint = device_verification_endpoint.to_string();
        self
    }

    /// Sets the path of the client registration endpoint.
    ///
    /// # Arguments
    ///
    /// * `client_registration_endpoint` - The path of the endpoint.
    pub fn client_registration_endpoint(mut self, client_registration_endpoint: &str) -> Self {
        self.settings.client_registration_endpoint = client_registration_endpoint.to_string();
        self
    }

    /// Sets the token a client presents to register itself.
    ///
    /// # Arguments
    ///
    /// * `client_registration_initial_access_token` - The initial access token
    ///   of the endpoint.
    pub fn client_registration_initial_access_token(
        mut self,
        client_registration_initial_access_token: &str,
    ) -> Self {
        self.settings.client_registration_initial_access_token =
            Some(client_registration_initial_access_token.to_string());
        self
    }

    /// Sets whether any client may register itself.
    ///
    /// # Arguments
    ///
    /// * `allow_open_client_registration` - Whether registration is open.
    pub fn allow_open_client_registration(mut self, allow_open_client_registration: bool) -> Self {
        self.settings.allow_open_client_registration = allow_open_client_registration;
        self
    }

    /// Sets whether an authorization request has to be pushed.
    ///
    /// # Arguments
    ///
    /// * `require_pushed_authorization_requests` - Whether the authorization
    ///   endpoint serves only a request that names a pushed request URI.
    pub fn require_pushed_authorization_requests(
        mut self,
        require_pushed_authorization_requests: bool,
    ) -> Self {
        self.settings.require_pushed_authorization_requests =
            require_pushed_authorization_requests;
        self
    }

    /// Sets whether the certificate of a client may be read from a forwarded
    /// header.
    ///
    /// # Arguments
    ///
    /// * `trust_forwarded_client_certificate` - Whether the header may be
    ///   trusted.
    pub fn trust_forwarded_client_certificate(
        mut self,
        trust_forwarded_client_certificate: bool,
    ) -> Self {
        self.settings.trust_forwarded_client_certificate = trust_forwarded_client_certificate;
        self
    }

    /// Builds the settings.
    pub fn build(self) -> AuthorizationServerSettings {
        self.settings
    }
}

#[cfg(test)]
mod tests {
    use super::AuthorizationServerSettings;

    #[test]
    fn the_default_paths_are_the_paths_of_the_specification() {
        let settings = AuthorizationServerSettings::default();

        assert_eq!(settings.authorization_endpoint(), "/oauth2/authorize");
        assert_eq!(settings.token_endpoint(), "/oauth2/token");
        assert_eq!(settings.jwk_set_endpoint(), "/oauth2/jwks");
        assert_eq!(settings.revocation_endpoint(), "/oauth2/revoke");
        assert_eq!(settings.introspection_endpoint(), "/oauth2/introspect");
        assert_eq!(settings.issuer(), None);
    }

    #[test]
    fn the_metadata_names_the_endpoints_of_the_server() {
        let settings = AuthorizationServerSettings::builder()
            .issuer("https://server.example/")
            .scopes(["read"])
            .build();

        let metadata = settings.metadata();

        assert_eq!(metadata["issuer"], "https://server.example");
        assert_eq!(
            metadata["authorization_endpoint"],
            "https://server.example/oauth2/authorize"
        );
        assert_eq!(
            metadata["token_endpoint"],
            "https://server.example/oauth2/token"
        );
        assert_eq!(metadata["grant_types_supported"][0], "authorization_code");
        assert_eq!(metadata["scopes_supported"][0], "read");
    }
}

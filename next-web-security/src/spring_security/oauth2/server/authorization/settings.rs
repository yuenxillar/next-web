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

    /// Returns the scopes the server advertises, if any are configured.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Returns the metadata document of the authorization server.
    ///
    /// The document is the `oauth-authorization-server` metadata of the
    /// specification, so a client can discover the endpoints of the server.
    pub fn metadata(&self) -> Value {
        let issuer = self.issuer_or("");
        let mut metadata = json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{issuer}{}", self.authorization_endpoint),
            "token_endpoint": format!("{issuer}{}", self.token_endpoint),
            "jwks_uri": format!("{issuer}{}", self.jwk_set_endpoint),
            "revocation_endpoint": format!("{issuer}{}", self.revocation_endpoint),
            "introspection_endpoint": format!("{issuer}{}", self.introspection_endpoint),
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "client_credentials", "refresh_token"],
            "token_endpoint_auth_methods_supported": [
                "client_secret_basic",
                "client_secret_post",
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

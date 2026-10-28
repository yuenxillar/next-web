//! The tokens the authorization server issues.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;

use crate::oauth2::jose::JwtSigner;

use super::{
    client::{AuthorizationGrantType, RegisteredClient},
    error::OAuth2Error,
};

/// The type of a token.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum OAuth2TokenType {
    /// An access token.
    AccessToken,
    /// A refresh token.
    RefreshToken,
    /// An authorization code.
    AuthorizationCode,
    /// An identifier token.
    IdToken,
    /// The state of an authorization request.
    State,
}

impl OAuth2TokenType {
    /// Returns the value of the type.
    pub fn value(&self) -> &'static str {
        match self {
            Self::AccessToken => "access_token",
            Self::RefreshToken => "refresh_token",
            Self::AuthorizationCode => "authorization_code",
            Self::IdToken => "id_token",
            Self::State => "state",
        }
    }
}

/// An access token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2AccessToken {
    token_value: String,
    token_type: String,
    issued_at: u64,
    expires_at: u64,
    scopes: Vec<String>,
}

impl OAuth2AccessToken {
    /// Creates an access token.
    ///
    /// # Arguments
    ///
    /// * `token_value` - The value of the token.
    /// * `issued_at` - The time the token was issued, in seconds since the
    ///   epoch.
    /// * `expires_at` - The time the token expires, in seconds since the epoch.
    /// * `scopes` - The scopes the token was issued for.
    pub fn new(
        token_value: impl Into<String>,
        issued_at: u64,
        expires_at: u64,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            token_value: token_value.into(),
            token_type: "Bearer".to_string(),
            issued_at,
            expires_at,
            scopes,
        }
    }

    /// Returns the value of the token.
    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    /// Returns the type of the token.
    pub fn token_type(&self) -> &str {
        &self.token_type
    }

    /// Returns the time the token was issued, in seconds since the epoch.
    pub fn issued_at(&self) -> u64 {
        self.issued_at
    }

    /// Returns the time the token expires, in seconds since the epoch.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Returns the scopes the token was issued for.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Returns whether the token is expired at the given time.
    ///
    /// # Arguments
    ///
    /// * `now` - The time to compare with, in seconds since the epoch.
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

/// A refresh token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2RefreshToken {
    token_value: String,
    issued_at: u64,
    expires_at: u64,
}

impl OAuth2RefreshToken {
    /// Creates a refresh token.
    ///
    /// # Arguments
    ///
    /// * `token_value` - The value of the token.
    /// * `issued_at` - The time the token was issued, in seconds since the
    ///   epoch.
    /// * `expires_at` - The time the token expires, in seconds since the epoch.
    pub fn new(token_value: impl Into<String>, issued_at: u64, expires_at: u64) -> Self {
        Self {
            token_value: token_value.into(),
            issued_at,
            expires_at,
        }
    }

    /// Returns the value of the token.
    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    /// Returns the time the token was issued, in seconds since the epoch.
    pub fn issued_at(&self) -> u64 {
        self.issued_at
    }

    /// Returns the time the token expires, in seconds since the epoch.
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Returns whether the token is expired at the given time.
    ///
    /// # Arguments
    ///
    /// * `now` - The time to compare with, in seconds since the epoch.
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

/// A token the authorization server issued.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OAuth2Token {
    /// An access token.
    Access(OAuth2AccessToken),
    /// A refresh token.
    Refresh(OAuth2RefreshToken),
}

impl OAuth2Token {
    /// Returns the type of the token.
    pub fn token_type(&self) -> OAuth2TokenType {
        match self {
            Self::Access(_) => OAuth2TokenType::AccessToken,
            Self::Refresh(_) => OAuth2TokenType::RefreshToken,
        }
    }

    /// Returns the value of the token.
    pub fn token_value(&self) -> &str {
        match self {
            Self::Access(token) => token.token_value(),
            Self::Refresh(token) => token.token_value(),
        }
    }

    /// Returns the time the token was issued, in seconds since the epoch.
    pub fn issued_at(&self) -> u64 {
        match self {
            Self::Access(token) => token.issued_at(),
            Self::Refresh(token) => token.issued_at(),
        }
    }

    /// Returns the time the token expires, in seconds since the epoch.
    pub fn expires_at(&self) -> u64 {
        match self {
            Self::Access(token) => token.expires_at(),
            Self::Refresh(token) => token.expires_at(),
        }
    }
}

/// The values a token is generated from.
#[derive(Clone, Debug)]
pub struct OAuth2TokenContext {
    token_type: OAuth2TokenType,
    registered_client: RegisteredClient,
    principal_name: Option<String>,
    scopes: Vec<String>,
    issuer: String,
    authorization_id: Option<String>,
}

impl OAuth2TokenContext {
    /// Creates a token context.
    ///
    /// # Arguments
    ///
    /// * `token_type` - The type of the token to generate.
    /// * `registered_client` - The client the token is issued to.
    /// * `issuer` - The issuer of the authorization server.
    pub fn new(
        token_type: OAuth2TokenType,
        registered_client: RegisteredClient,
        issuer: impl Into<String>,
    ) -> Self {
        Self {
            token_type,
            registered_client,
            principal_name: None,
            scopes: Vec::new(),
            issuer: issuer.into(),
            authorization_id: None,
        }
    }

    /// Sets the name of the principal the token is issued for.
    ///
    /// # Arguments
    ///
    /// * `principal_name` - The name of the principal.
    pub fn set_principal_name(mut self, principal_name: impl Into<String>) -> Self {
        self.principal_name = Some(principal_name.into());
        self
    }

    /// Sets the scopes of the token.
    ///
    /// # Arguments
    ///
    /// * `scopes` - The scopes of the token.
    pub fn set_scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the identifier of the authorization the token belongs to.
    ///
    /// # Arguments
    ///
    /// * `authorization_id` - The identifier of the authorization.
    pub fn set_authorization_id(mut self, authorization_id: impl Into<String>) -> Self {
        self.authorization_id = Some(authorization_id.into());
        self
    }

    /// Returns the type of the token to generate.
    pub fn token_type(&self) -> OAuth2TokenType {
        self.token_type
    }

    /// Returns the client the token is issued to.
    pub fn registered_client(&self) -> &RegisteredClient {
        &self.registered_client
    }

    /// Returns the name of the principal the token is issued for.
    pub fn principal_name(&self) -> Option<&str> {
        self.principal_name.as_deref()
    }

    /// Returns the scopes of the token.
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    /// Returns the issuer of the authorization server.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Returns the identifier of the authorization the token belongs to.
    pub fn authorization_id(&self) -> Option<&str> {
        self.authorization_id.as_deref()
    }

    /// Returns the grant of the client the token is issued for.
    pub fn grant_type(&self) -> Option<AuthorizationGrantType> {
        self.registered_client
            .authorization_grant_types()
            .first()
            .copied()
    }
}

/// Generates the tokens of an authorization server.
pub trait OAuth2TokenGenerator: Send + Sync {
    /// Generates a token for the given context.
    ///
    /// Returns `None` when the generator does not handle the type of the
    /// context, which is what lets a delegating generator ask the next one.
    ///
    /// # Arguments
    ///
    /// * `context` - The values the token is generated from.
    fn generate(&self, context: &OAuth2TokenContext) -> Result<Option<OAuth2Token>, OAuth2Error>;
}

/// Generates an opaque access token.
#[derive(Clone, Debug, Default)]
pub struct OAuth2AccessTokenGenerator;

impl OAuth2TokenGenerator for OAuth2AccessTokenGenerator {
    fn generate(&self, context: &OAuth2TokenContext) -> Result<Option<OAuth2Token>, OAuth2Error> {
        if context.token_type() != OAuth2TokenType::AccessToken {
            return Ok(None);
        }

        let issued_at = now();
        let time_to_live = context
            .registered_client()
            .token_settings()
            .access_token_time_to_live();
        Ok(Some(OAuth2Token::Access(OAuth2AccessToken::new(
            next_web_core::Uuid::new_v4().to_string(),
            issued_at,
            issued_at + time_to_live,
            context.scopes().to_vec(),
        ))))
    }
}

/// Generates a refresh token.
#[derive(Clone, Debug, Default)]
pub struct OAuth2RefreshTokenGenerator;

impl OAuth2TokenGenerator for OAuth2RefreshTokenGenerator {
    fn generate(&self, context: &OAuth2TokenContext) -> Result<Option<OAuth2Token>, OAuth2Error> {
        if context.token_type() != OAuth2TokenType::RefreshToken {
            return Ok(None);
        }

        let issued_at = now();
        let time_to_live = context
            .registered_client()
            .token_settings()
            .refresh_token_time_to_live();
        Ok(Some(OAuth2Token::Refresh(OAuth2RefreshToken::new(
            next_web_core::Uuid::new_v4().to_string(),
            issued_at,
            issued_at + time_to_live,
        ))))
    }
}

/// Generates an access token that is a JSON Web Token signed by the
/// authorization server.
#[derive(Clone)]
pub struct JwtGenerator {
    signer: Arc<dyn JwtSigner>,
}

impl JwtGenerator {
    /// Creates a generator from the signer of the authorization server.
    ///
    /// # Arguments
    ///
    /// * `signer` - The signer the tokens are signed with.
    pub fn new(signer: Arc<dyn JwtSigner>) -> Self {
        Self { signer }
    }
}

impl OAuth2TokenGenerator for JwtGenerator {
    fn generate(&self, context: &OAuth2TokenContext) -> Result<Option<OAuth2Token>, OAuth2Error> {
        if context.token_type() != OAuth2TokenType::AccessToken {
            return Ok(None);
        }

        let issued_at = now();
        let time_to_live = context
            .registered_client()
            .token_settings()
            .access_token_time_to_live();
        let expires_at = issued_at + time_to_live;

        let mut claims = json!({
            "iss": context.issuer(),
            "aud": context.registered_client().client_id(),
            "iat": issued_at,
            "exp": expires_at,
            "jti": next_web_core::Uuid::new_v4().to_string(),
        });
        if let Some(principal_name) = context.principal_name() {
            claims["sub"] = json!(principal_name);
        } else {
            claims["sub"] = json!(context.registered_client().client_id());
        }
        if !context.scopes().is_empty() {
            claims["scope"] = json!(context.scopes().join(" "));
        }
        if let Some(authorization_id) = context.authorization_id() {
            claims["authorization_id"] = json!(authorization_id);
        }

        let token_value = self.signer.sign(&claims)?;
        Ok(Some(OAuth2Token::Access(OAuth2AccessToken::new(
            token_value,
            issued_at,
            expires_at,
            context.scopes().to_vec(),
        ))))
    }
}

/// Delegates the generation of a token to the generators it holds, in order.
#[derive(Clone, Default)]
pub struct DelegatingOAuth2TokenGenerator {
    generators: Vec<Arc<dyn OAuth2TokenGenerator>>,
}

impl DelegatingOAuth2TokenGenerator {
    /// Creates a generator from the generators it delegates to.
    ///
    /// # Arguments
    ///
    /// * `generators` - The generators, in the order they are asked.
    pub fn new<I>(generators: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn OAuth2TokenGenerator>>,
    {
        Self {
            generators: generators.into_iter().collect(),
        }
    }

    /// Adds a generator to the delegation.
    ///
    /// # Arguments
    ///
    /// * `generator` - The generator to add.
    pub fn add_generator(&mut self, generator: Arc<dyn OAuth2TokenGenerator>) {
        self.generators.push(generator);
    }
}

impl OAuth2TokenGenerator for DelegatingOAuth2TokenGenerator {
    fn generate(&self, context: &OAuth2TokenContext) -> Result<Option<OAuth2Token>, OAuth2Error> {
        for generator in &self.generators {
            if let Some(token) = generator.generate(context)? {
                return Ok(Some(token));
            }
        }

        Ok(None)
    }
}

/// Returns the current time in seconds since the epoch.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::{
        DelegatingOAuth2TokenGenerator, JwtGenerator, OAuth2AccessTokenGenerator,
        OAuth2RefreshTokenGenerator, OAuth2Token, OAuth2TokenContext, OAuth2TokenGenerator,
        OAuth2TokenType,
    };
    use crate::oauth2::{
        jose::{base64_url_decode, JwtSigner, SecretJwtSigner},
        server::authorization::client::RegisteredClient,
    };

    fn client() -> RegisteredClient {
        RegisteredClient::with_client_id("demo-client")
    }

    #[test]
    fn the_jwt_generator_signs_a_token_with_the_claims_of_the_context() {
        let signer: Arc<dyn JwtSigner> = Arc::new(SecretJwtSigner::new("secret"));
        let generator = JwtGenerator::new(signer);
        let context = OAuth2TokenContext::new(
            OAuth2TokenType::AccessToken,
            client(),
            "https://server.example",
        )
        .set_principal_name("user")
        .set_scopes(["read"]);

        let token = generator
            .generate(&context)
            .expect("the token is generated")
            .expect("the context is handled");

        let OAuth2Token::Access(token) = token else {
            panic!("the generator produces an access token");
        };
        let payload = token
            .token_value()
            .split('.')
            .nth(1)
            .and_then(base64_url_decode)
            .expect("the token is a JWT");
        let claims: serde_json::Value =
            serde_json::from_slice(&payload).expect("the payload of the token is JSON");
        assert_eq!(claims["iss"], "https://server.example");
        assert_eq!(claims["sub"], "user");
        assert_eq!(claims["aud"], "demo-client");
        assert_eq!(claims["scope"], "read");
        assert_eq!(token.scopes(), ["read"]);
        assert!(claims["exp"].as_u64().unwrap() > claims["iat"].as_u64().unwrap());
    }

    #[test]
    fn a_generator_that_does_not_handle_the_type_returns_none() {
        let generator = OAuth2RefreshTokenGenerator;
        let context = OAuth2TokenContext::new(
            OAuth2TokenType::AccessToken,
            client(),
            "https://server.example",
        );

        assert!(generator.generate(&context).expect("no error").is_none());
    }

    #[test]
    fn the_delegating_generator_asks_its_generators_in_order() {
        let generator = DelegatingOAuth2TokenGenerator::new([
            Arc::new(OAuth2RefreshTokenGenerator) as Arc<dyn OAuth2TokenGenerator>,
            Arc::new(OAuth2AccessTokenGenerator) as Arc<dyn OAuth2TokenGenerator>,
        ]);

        let access = generator
            .generate(&OAuth2TokenContext::new(
                OAuth2TokenType::AccessToken,
                client(),
                "https://server.example",
            ))
            .expect("no error")
            .expect("the access token generator handles the context");
        let refresh = generator
            .generate(&OAuth2TokenContext::new(
                OAuth2TokenType::RefreshToken,
                client(),
                "https://server.example",
            ))
            .expect("no error")
            .expect("the refresh token generator handles the context");

        assert_eq!(access.token_type(), OAuth2TokenType::AccessToken);
        assert_eq!(refresh.token_type(), OAuth2TokenType::RefreshToken);
        assert!(matches!(access, OAuth2Token::Access(_)));
        assert!(matches!(refresh, OAuth2Token::Refresh(_)));
        assert_eq!(
            json!(access.token_value()).as_str().unwrap().is_empty(),
            false
        );
    }
}

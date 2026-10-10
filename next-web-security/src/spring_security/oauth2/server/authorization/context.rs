//! The values the endpoints of the authorization server are served with.

use std::sync::Arc;

use crate::crypto::password::PasswordEncoder;

use super::{
    authorization::OAuth2AuthorizationService,
    client::RegisteredClientRepository,
    jose::{JwkSource, SecretJwkSource},
    settings::AuthorizationServerSettings,
    token::{
        DelegatingOAuth2TokenGenerator, JwtGenerator, OAuth2RefreshTokenGenerator,
        OAuth2TokenGenerator,
    },
};

/// The issuer the tokens and the metadata of the server are built with when the
/// settings do not name one.
pub const DEFAULT_ISSUER: &str = "http://localhost";

/// The default lifetime of an authorization code, in seconds.
pub const DEFAULT_AUTHORIZATION_CODE_TIME_TO_LIVE: u64 = 300;

/// The values every endpoint of the authorization server is served with.
///
/// The context is shared by the filters of the endpoints, so a filter does not
/// have to know how the values were contributed.
#[derive(Clone)]
pub struct AuthorizationServerContext {
    settings: AuthorizationServerSettings,
    registered_clients: Arc<dyn RegisteredClientRepository>,
    authorizations: Arc<dyn OAuth2AuthorizationService>,
    token_generator: Arc<dyn OAuth2TokenGenerator>,
    jwk_source: Option<Arc<dyn JwkSource>>,
    password_encoder: Option<Arc<dyn PasswordEncoder>>,
    authorization_code_time_to_live: u64,
}

impl AuthorizationServerContext {
    /// Creates a builder for the context.
    pub fn builder() -> AuthorizationServerContextBuilder {
        AuthorizationServerContextBuilder::default()
    }

    /// Returns the settings of the authorization server.
    pub fn settings(&self) -> &AuthorizationServerSettings {
        &self.settings
    }

    /// Returns the repository of the registered clients.
    pub fn registered_clients(&self) -> &Arc<dyn RegisteredClientRepository> {
        &self.registered_clients
    }

    /// Returns the service that stores the authorizations.
    pub fn authorizations(&self) -> &Arc<dyn OAuth2AuthorizationService> {
        &self.authorizations
    }

    /// Returns the generator of the tokens of the server.
    pub fn token_generator(&self) -> &Arc<dyn OAuth2TokenGenerator> {
        &self.token_generator
    }

    /// Returns the keys the tokens of the server are signed with, if any are
    /// configured.
    pub fn jwk_source(&self) -> Option<&Arc<dyn JwkSource>> {
        self.jwk_source.as_ref()
    }

    /// Returns the encoder the secrets of the clients are matched with, if one
    /// is configured.
    pub fn password_encoder(&self) -> Option<&Arc<dyn PasswordEncoder>> {
        self.password_encoder.as_ref()
    }

    /// Returns the lifetime of an authorization code, in seconds.
    pub fn authorization_code_time_to_live(&self) -> u64 {
        self.authorization_code_time_to_live
    }

    /// Returns the issuer of the authorization server.
    pub fn issuer(&self) -> String {
        self.settings.issuer_or(DEFAULT_ISSUER)
    }
}

/// Builds an [`AuthorizationServerContext`].
#[derive(Default)]
pub struct AuthorizationServerContextBuilder {
    settings: Option<AuthorizationServerSettings>,
    registered_clients: Option<Arc<dyn RegisteredClientRepository>>,
    authorizations: Option<Arc<dyn OAuth2AuthorizationService>>,
    token_generator: Option<Arc<dyn OAuth2TokenGenerator>>,
    jwk_source: Option<Arc<dyn JwkSource>>,
    password_encoder: Option<Arc<dyn PasswordEncoder>>,
    authorization_code_time_to_live: Option<u64>,
}

impl AuthorizationServerContextBuilder {
    /// Sets the settings of the server.
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings of the server.
    pub fn settings(mut self, settings: AuthorizationServerSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Sets the repository of the registered clients.
    ///
    /// # Arguments
    ///
    /// * `registered_clients` - The repository of the registered clients.
    pub fn registered_clients(
        mut self,
        registered_clients: Arc<dyn RegisteredClientRepository>,
    ) -> Self {
        self.registered_clients = Some(registered_clients);
        self
    }

    /// Sets the service that stores the authorizations.
    ///
    /// # Arguments
    ///
    /// * `authorizations` - The service that stores the authorizations.
    pub fn authorizations(mut self, authorizations: Arc<dyn OAuth2AuthorizationService>) -> Self {
        self.authorizations = Some(authorizations);
        self
    }

    /// Sets the generator of the tokens of the server.
    ///
    /// # Arguments
    ///
    /// * `token_generator` - The generator of the tokens.
    pub fn token_generator(mut self, token_generator: Arc<dyn OAuth2TokenGenerator>) -> Self {
        self.token_generator = Some(token_generator);
        self
    }

    /// Sets the keys the tokens of the server are signed with.
    ///
    /// # Arguments
    ///
    /// * `jwk_source` - The keys of the server.
    pub fn jwk_source(mut self, jwk_source: Arc<dyn JwkSource>) -> Self {
        self.jwk_source = Some(jwk_source);
        self
    }

    /// Sets the encoder the secrets of the clients are matched with.
    ///
    /// # Arguments
    ///
    /// * `password_encoder` - The encoder of the application.
    pub fn password_encoder(mut self, password_encoder: Arc<dyn PasswordEncoder>) -> Self {
        self.password_encoder = Some(password_encoder);
        self
    }

    /// Sets the lifetime of an authorization code, in seconds.
    ///
    /// # Arguments
    ///
    /// * `authorization_code_time_to_live` - The lifetime in seconds.
    pub fn authorization_code_time_to_live(mut self, authorization_code_time_to_live: u64) -> Self {
        self.authorization_code_time_to_live = Some(authorization_code_time_to_live);
        self
    }

    /// Builds the context.
    ///
    /// The context needs a repository of registered clients: without one, no
    /// client can be authenticated, so building the context fails while the
    /// application is configured.
    pub fn build(self) -> AuthorizationServerContext {
        let registered_clients = self.registered_clients.expect(
            "RegisteredClientRepository is required to serve the authorization server endpoints",
        );
        let settings = self.settings.unwrap_or_default();

        let jwk_source = self.jwk_source.or_else(|| {
            let secret = next_web_core::Uuid::new_v4().to_string();
            tracing::warn!(
                "The authorization server signs its tokens with a key that is generated at \
                 startup. Configure a JWK source with a stable key so the tokens stay valid \
                 across a restart."
            );
            Some(Arc::new(SecretJwkSource::new(&secret)) as Arc<dyn JwkSource>)
        });

        let token_generator = self.token_generator.unwrap_or_else(|| {
            let jwt_generator = Arc::new(JwtGenerator::new(
                jwk_source
                    .clone()
                    .expect("a key source is configured above"),
            )) as Arc<dyn OAuth2TokenGenerator>;
            Arc::new(DelegatingOAuth2TokenGenerator::new([
                jwt_generator,
                Arc::new(OAuth2RefreshTokenGenerator) as Arc<dyn OAuth2TokenGenerator>,
            ])) as Arc<dyn OAuth2TokenGenerator>
        });

        AuthorizationServerContext {
            settings,
            registered_clients,
            authorizations: self.authorizations.unwrap_or_else(|| {
                Arc::new(super::authorization::InMemoryOAuth2AuthorizationService::default())
            }),
            token_generator,
            jwk_source,
            password_encoder: self.password_encoder,
            authorization_code_time_to_live: self
                .authorization_code_time_to_live
                .unwrap_or(DEFAULT_AUTHORIZATION_CODE_TIME_TO_LIVE),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{AuthorizationServerContext, DEFAULT_ISSUER};
    use crate::oauth2::server::authorization::{
        client::InMemoryRegisteredClientRepository, token::OAuth2TokenType,
    };

    #[test]
    fn a_context_needs_a_repository_of_registered_clients() {
        let result = std::panic::catch_unwind(|| AuthorizationServerContext::builder().build());

        assert!(result.is_err());
    }

    #[test]
    fn a_context_falls_back_to_a_generated_key_and_a_jwt_generator() {
        let context = AuthorizationServerContext::builder()
            .registered_clients(Arc::new(InMemoryRegisteredClientRepository::default()))
            .build();

        assert!(context.jwk_source().is_some());
        assert_eq!(context.issuer(), DEFAULT_ISSUER);
        assert!(context
            .token_generator()
            .generate(
                &crate::oauth2::server::authorization::token::OAuth2TokenContext::new(
                    OAuth2TokenType::AccessToken,
                    crate::oauth2::server::authorization::client::RegisteredClient::with_client_id(
                        "demo-client",
                    ),
                    context.issuer(),
                )
            )
            .expect("the token is generated")
            .is_some());
    }
}

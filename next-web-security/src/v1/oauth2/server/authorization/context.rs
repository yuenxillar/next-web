//! The values the endpoints of the authorization server are served with.

use std::sync::Arc;

use crate::{
    crypto::password::PasswordEncoder,
    oauth2::jose::{JwkSource, JwtSigner, SecretJwtSigner, SignerJwkSource},
};

use super::{
    authorization::OAuth2AuthorizationService,
    client::RegisteredClientRepository,
    consent::{InMemoryOAuth2AuthorizationConsentService, OAuth2AuthorizationConsentService},
    device::{
        InMemoryOAuth2DeviceAuthorizationService, OAuth2DeviceAuthorizationService,
    },
    pushed_authorization_request::{
        InMemoryOAuth2PushedAuthorizationRequestRepository,
        OAuth2PushedAuthorizationRequestRepository,
    },
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
    authorization_consents: Arc<dyn OAuth2AuthorizationConsentService>,
    pushed_authorization_requests: Arc<dyn OAuth2PushedAuthorizationRequestRepository>,
    device_authorizations: Arc<dyn OAuth2DeviceAuthorizationService>,
    token_generator: Arc<dyn OAuth2TokenGenerator>,
    jwt_signer: Arc<dyn JwtSigner>,
    password_encoder: Option<Arc<dyn PasswordEncoder>>,
    consent_page: Option<String>,
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

    /// Returns the service that stores the consents of the resource owners.
    pub fn authorization_consents(&self) -> &Arc<dyn OAuth2AuthorizationConsentService> {
        &self.authorization_consents
    }

    /// Returns the repository of the authorization requests the clients
    /// pushed.
    pub fn pushed_authorization_requests(
        &self,
    ) -> &Arc<dyn OAuth2PushedAuthorizationRequestRepository> {
        &self.pushed_authorization_requests
    }

    /// Returns the service that stores the device authorizations.
    pub fn device_authorizations(&self) -> &Arc<dyn OAuth2DeviceAuthorizationService> {
        &self.device_authorizations
    }

    /// Returns the generator of the tokens of the server.
    pub fn token_generator(&self) -> &Arc<dyn OAuth2TokenGenerator> {
        &self.token_generator
    }

    /// Returns the signer the tokens of the server are signed with.
    pub fn jwt_signer(&self) -> &Arc<dyn JwtSigner> {
        &self.jwt_signer
    }

    /// Returns the keys the tokens of the server are signed with.
    ///
    /// The keys are the public keys of the signer, which is what the JWK set
    /// endpoint publishes and what a resource server verifies the tokens with.
    pub fn jwk_source(&self) -> Arc<dyn JwkSource> {
        Arc::new(SignerJwkSource::new(self.jwt_signer.clone()))
    }

    /// Returns the encoder the secrets of the clients are matched with, if one
    /// is configured.
    pub fn password_encoder(&self) -> Option<&Arc<dyn PasswordEncoder>> {
        self.password_encoder.as_ref()
    }

    /// Returns the page the resource owner is asked for consent on, if one is
    /// configured.
    ///
    /// Consent is asked only when a page is configured, which is what lets an
    /// application that does not serve a consent page grant the scopes of its
    /// clients without asking.
    pub fn consent_page(&self) -> Option<&str> {
        self.consent_page.as_deref()
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
    authorization_consents: Option<Arc<dyn OAuth2AuthorizationConsentService>>,
    pushed_authorization_requests: Option<Arc<dyn OAuth2PushedAuthorizationRequestRepository>>,
    device_authorizations: Option<Arc<dyn OAuth2DeviceAuthorizationService>>,
    token_generator: Option<Arc<dyn OAuth2TokenGenerator>>,
    jwt_signer: Option<Arc<dyn JwtSigner>>,
    password_encoder: Option<Arc<dyn PasswordEncoder>>,
    consent_page: Option<String>,
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

    /// Sets the service that stores the consents of the resource owners.
    ///
    /// # Arguments
    ///
    /// * `authorization_consents` - The service that stores the consents.
    pub fn authorization_consents(
        mut self,
        authorization_consents: Arc<dyn OAuth2AuthorizationConsentService>,
    ) -> Self {
        self.authorization_consents = Some(authorization_consents);
        self
    }

    /// Sets the repository of the authorization requests the clients pushed.
    ///
    /// # Arguments
    ///
    /// * `pushed_authorization_requests` - The repository of the requests.
    pub fn pushed_authorization_requests(
        mut self,
        pushed_authorization_requests: Arc<dyn OAuth2PushedAuthorizationRequestRepository>,
    ) -> Self {
        self.pushed_authorization_requests = Some(pushed_authorization_requests);
        self
    }

    /// Sets the service that stores the device authorizations.
    ///
    /// # Arguments
    ///
    /// * `device_authorizations` - The service that stores the authorizations.
    pub fn device_authorizations(
        mut self,
        device_authorizations: Arc<dyn OAuth2DeviceAuthorizationService>,
    ) -> Self {
        self.device_authorizations = Some(device_authorizations);
        self
    }

    /// Sets the page the resource owner is asked for consent on.
    ///
    /// # Arguments
    ///
    /// * `consent_page` - The path of the page of the application.
    pub fn consent_page(mut self, consent_page: impl Into<String>) -> Self {
        self.consent_page = Some(consent_page.into());
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

    /// Sets the signer the tokens of the server are signed with.
    ///
    /// # Arguments
    ///
    /// * `jwt_signer` - The signer of the server.
    pub fn jwt_signer(mut self, jwt_signer: Arc<dyn JwtSigner>) -> Self {
        self.jwt_signer = Some(jwt_signer);
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

        let jwt_signer = self.jwt_signer.unwrap_or_else(|| {
            let secret = next_web_core::Uuid::new_v4().to_string();
            tracing::warn!(
                "The authorization server signs its tokens with a secret that is generated at \
                 startup. Configure a signer with a stable key so the tokens stay valid across a \
                 restart."
            );
            Arc::new(SecretJwtSigner::new(&secret)) as Arc<dyn JwtSigner>
        });

        let token_generator = self.token_generator.unwrap_or_else(|| {
            let jwt_generator =
                Arc::new(JwtGenerator::new(jwt_signer.clone())) as Arc<dyn OAuth2TokenGenerator>;
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
            authorization_consents: self.authorization_consents.unwrap_or_else(|| {
                Arc::new(InMemoryOAuth2AuthorizationConsentService::default())
                    as Arc<dyn OAuth2AuthorizationConsentService>
            }),
            pushed_authorization_requests: self.pushed_authorization_requests.unwrap_or_else(|| {
                Arc::new(InMemoryOAuth2PushedAuthorizationRequestRepository::default())
                    as Arc<dyn OAuth2PushedAuthorizationRequestRepository>
            }),
            device_authorizations: self.device_authorizations.unwrap_or_else(|| {
                Arc::new(InMemoryOAuth2DeviceAuthorizationService::default())
                    as Arc<dyn OAuth2DeviceAuthorizationService>
            }),
            token_generator,
            jwt_signer,
            password_encoder: self.password_encoder,
            consent_page: self.consent_page,
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

        assert_eq!(context.jwk_source().jwk_set().keys().len(), 1);
        assert_eq!(context.jwt_signer().jwk_set().keys().len(), 1);
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

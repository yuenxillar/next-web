//! Configures the endpoints of the OAuth 2.0 authorization server.

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{BaseHttpConfigurer, CsrfConfigurer},
            http_security_builder::HttpSecurityBuilder,
        },
    },
    crypto::password::PasswordEncoder,
    oauth2::server::authorization::{
        AuthorizationEndpointFilter, AuthorizationServerContext,
        AuthorizationServerMetadataEndpointFilter, AuthorizationServerSettings,
        InMemoryOAuth2AuthorizationService, InMemoryRegisteredClientRepository,
        JwkSetEndpointFilter, JwtSigner, LogoutEndpointFilter, OAuth2AuthorizationConsentService,
        OAuth2AuthorizationService, OAuth2ClientRegistrationEndpointFilter,
        OAuth2DeviceAuthorizationEndpointFilter, OAuth2DeviceVerificationEndpointFilter,
        OAuth2PushedAuthorizationRequestEndpointFilter, OAuth2TokenGenerator,
        RegisteredClientRepository, TokenEndpointFilter, TokenIntrospectionEndpointFilter,
        TokenRevocationEndpointFilter, UserInfoEndpointFilter,
    },
    web::{
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{PathPatternRequestMatcherBuilder, RequestMatcher},
    },
};

/// Configures the endpoints of the OAuth 2.0 authorization server.
///
/// The configurer adds the filters of the endpoints to the HTTP security, so
/// that the security filter chain of the application serves them:
///
/// * the metadata endpoint and the JWK set endpoint, which a client reads to
///   discover the server,
/// * the authorization endpoint, which grants an authorization code to a client
///   after the resource owner has logged in,
/// * the token endpoint, which exchanges the code, a refresh token, or the
///   credentials of a client for an access token,
/// * the introspection endpoint and the revocation endpoint, which a resource
///   server uses to manage a token.
///
/// The values the endpoints are served with are taken from the configurer, and
/// the values the application contributed to its context are used for the ones
/// that were not configured here. A repository of registered clients is
/// required, and the authorization code grant renews the CSRF protection of the
/// endpoints, because a client authenticates with its own credentials rather
/// than with a CSRF token.
///
/// # Examples
///
/// ```ignore
/// #[singleton(binds = [Self::into_web_security_configure])]
/// #[derive(Clone)]
/// struct AuthorizationServerConfigurer;
///
/// impl WebSecurityConfigurer for AuthorizationServerConfigurer {
///     fn configure(&mut self, http: &mut HttpSecurity) {
///         http.oauth2_authorization_server(|authorization_server| {
///             authorization_server
///                 .registered_client_repository(Arc::new(InMemoryRegisteredClientRepository::new([
///                     RegisteredClient::with_client_id("demo-client")
///                         .client_secret("{noop}secret")
///                         .redirect_uri("https://client.example/callback")
///                         .scope("read"),
///                 ])))
///                 .authorization_server_settings(
///                     AuthorizationServerSettings::builder()
///                         .issuer("https://server.example")
///                         .build(),
///                 );
///         });
///     }
/// }
/// ```
#[derive(Clone)]
pub struct OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    settings: Option<AuthorizationServerSettings>,
    registered_clients: Option<Arc<dyn RegisteredClientRepository>>,
    authorizations: Option<Arc<dyn OAuth2AuthorizationService>>,
    authorization_consents: Option<Arc<dyn OAuth2AuthorizationConsentService>>,
    token_generator: Option<Arc<dyn OAuth2TokenGenerator>>,
    jwt_signer: Option<Arc<dyn JwtSigner>>,
    password_encoder: Option<Arc<dyn PasswordEncoder>>,
    consent_page: Option<String>,
    authorization_code_time_to_live: Option<u64>,

    base: BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H>,
}

impl<H> OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    /// Sets the settings of the authorization server.
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings of the server.
    pub fn authorization_server_settings(
        &mut self,
        settings: AuthorizationServerSettings,
    ) -> &mut Self {
        self.settings = Some(settings);
        self
    }

    /// Sets the repository of the registered clients.
    ///
    /// # Arguments
    ///
    /// * `registered_clients` - The repository of the registered clients.
    pub fn registered_client_repository(
        &mut self,
        registered_clients: Arc<dyn RegisteredClientRepository>,
    ) -> &mut Self {
        self.registered_clients = Some(registered_clients);
        self
    }

    /// Sets the service that stores the authorizations.
    ///
    /// # Arguments
    ///
    /// * `authorizations` - The service that stores the authorizations.
    pub fn authorization_service(
        &mut self,
        authorizations: Arc<dyn OAuth2AuthorizationService>,
    ) -> &mut Self {
        self.authorizations = Some(authorizations);
        self
    }

    /// Sets the service that stores the consents of the resource owners.
    ///
    /// # Arguments
    ///
    /// * `authorization_consents` - The service that stores the consents.
    pub fn authorization_consent_service(
        &mut self,
        authorization_consents: Arc<dyn OAuth2AuthorizationConsentService>,
    ) -> &mut Self {
        self.authorization_consents = Some(authorization_consents);
        self
    }

    /// Sets the page the resource owner is asked for consent on.
    ///
    /// The application serves the page. Consent is asked only when the page is
    /// configured, and only for the clients that require it.
    ///
    /// # Arguments
    ///
    /// * `consent_page` - The path of the page of the application.
    pub fn consent_page(&mut self, consent_page: &str) -> &mut Self {
        assert!(
            !consent_page.trim().is_empty(),
            "consent_page cannot be empty"
        );
        self.consent_page = Some(consent_page.to_string());
        self
    }

    /// Sets the generator of the tokens of the server.
    ///
    /// # Arguments
    ///
    /// * `token_generator` - The generator of the tokens.
    pub fn token_generator(&mut self, token_generator: Arc<dyn OAuth2TokenGenerator>) -> &mut Self {
        self.token_generator = Some(token_generator);
        self
    }

    /// Sets the signer the tokens of the server are signed with.
    ///
    /// # Arguments
    ///
    /// * `jwt_signer` - The signer of the server. Its public keys are published
    ///   by the JWK set endpoint.
    pub fn jwt_signer(&mut self, jwt_signer: Arc<dyn JwtSigner>) -> &mut Self {
        self.jwt_signer = Some(jwt_signer);
        self
    }

    /// Sets the encoder the secrets of the clients are matched with.
    ///
    /// # Arguments
    ///
    /// * `password_encoder` - The encoder of the application.
    pub fn client_password_encoder(
        &mut self,
        password_encoder: Arc<dyn PasswordEncoder>,
    ) -> &mut Self {
        self.password_encoder = Some(password_encoder);
        self
    }

    /// Sets the lifetime of an authorization code, in seconds.
    ///
    /// # Arguments
    ///
    /// * `authorization_code_time_to_live` - The lifetime in seconds.
    pub fn authorization_code_time_to_live(
        &mut self,
        authorization_code_time_to_live: u64,
    ) -> &mut Self {
        self.authorization_code_time_to_live = Some(authorization_code_time_to_live);
        self
    }

    /// Returns the context the endpoints are served with.
    ///
    /// The values that were not configured here are taken from the context of
    /// the application, which is what lets an application contribute them as
    /// singletons instead.
    ///
    /// # Arguments
    ///
    /// * `http` - The HTTP security that shares the values of the application.
    fn authorization_server_context(&self, http: &H) -> AuthorizationServerContext {
        let settings = self
            .settings
            .clone()
            .or_else(|| http.shared_object::<AuthorizationServerSettings>().cloned())
            .unwrap_or_default();
        let registered_clients = self
            .registered_clients
            .clone()
            .or_else(|| {
                http.shared_object::<Arc<dyn RegisteredClientRepository>>()
                    .cloned()
            })
            .or_else(|| {
                tracing::debug!(
                    "No registered client was contributed to the context, so the authorization \
                     server has no client to authenticate"
                );
                Some(Arc::new(InMemoryRegisteredClientRepository::default())
                    as Arc<dyn RegisteredClientRepository>)
            })
            .expect("a repository of registered clients is configured above");
        let authorizations = self
            .authorizations
            .clone()
            .or_else(|| {
                http.shared_object::<Arc<dyn OAuth2AuthorizationService>>()
                    .cloned()
            })
            .unwrap_or_else(|| {
                Arc::new(InMemoryOAuth2AuthorizationService::default())
                    as Arc<dyn OAuth2AuthorizationService>
            });
        let token_generator = self.token_generator.clone().or_else(|| {
            http.shared_object::<Arc<dyn OAuth2TokenGenerator>>()
                .cloned()
        });
        let jwt_signer = self
            .jwt_signer
            .clone()
            .or_else(|| http.shared_object::<Arc<dyn JwtSigner>>().cloned());
        let authorization_consents = self.authorization_consents.clone().or_else(|| {
            http.shared_object::<Arc<dyn OAuth2AuthorizationConsentService>>()
                .cloned()
        });
        let password_encoder = self
            .password_encoder
            .clone()
            .or_else(|| http.shared_object::<Arc<dyn PasswordEncoder>>().cloned());

        let mut builder = AuthorizationServerContext::builder()
            .settings(settings)
            .registered_clients(registered_clients)
            .authorizations(authorizations);
        if let Some(token_generator) = token_generator {
            builder = builder.token_generator(token_generator);
        }
        if let Some(jwt_signer) = jwt_signer {
            builder = builder.jwt_signer(jwt_signer);
        }
        if let Some(authorization_consents) = authorization_consents {
            builder = builder.authorization_consents(authorization_consents);
        }
        if let Some(consent_page) = self.consent_page.as_ref() {
            builder = builder.consent_page(consent_page);
        }
        if let Some(password_encoder) = password_encoder {
            builder = builder.password_encoder(password_encoder);
        }
        if let Some(authorization_code_time_to_live) = self.authorization_code_time_to_live {
            builder = builder.authorization_code_time_to_live(authorization_code_time_to_live);
        }

        builder.build()
    }

    /// Renews the CSRF protection of the endpoints of the server.
    ///
    /// A client of the token, introspection, and revocation endpoints
    /// authenticates with its own credentials, and the consent of the resource
    /// owner and the answer to a device authorization are submitted to the
    /// endpoints of the server, so the endpoints are not protected by a CSRF
    /// token. An application that serves the consent page or the page of a
    /// device authorization itself protects those pages with its own rules.
    ///
    /// # Arguments
    ///
    /// * `http` - The HTTP security the configurer is applied to.
    /// * `context` - The context of the authorization server.
    fn ignore_csrf_protection(&self, http: &mut H, context: &AuthorizationServerContext) {
        let settings = context.settings();
        let matchers = [
            settings.token_endpoint(),
            settings.introspection_endpoint(),
            settings.revocation_endpoint(),
            settings.authorization_endpoint(),
            settings.pushed_authorization_request_endpoint(),
            settings.device_authorization_endpoint(),
            settings.device_verification_endpoint(),
            settings.client_registration_endpoint(),
        ]
        .into_iter()
        .map(ToString::to_string)
        // The registration endpoint manages a registered client under its path,
        // so the whole subtree of the endpoint is exempt.
        .chain([format!("{}/**", settings.client_registration_endpoint())])
        .map(|path| {
            Arc::new(PathPatternRequestMatcherBuilder::default().matcher(None, &path))
                as Arc<dyn RequestMatcher>
        })
        .collect::<Vec<_>>();

        if let Some(csrf) = http.configurer_mut::<CsrfConfigurer<H>>() {
            csrf.add_ignored_csrf_protection_matchers(matchers);
        }
    }
}

impl<H> Default for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            settings: None,
            registered_clients: None,
            authorizations: None,
            authorization_consents: None,
            token_generator: None,
            jwt_signer: None,
            password_encoder: None,
            consent_page: None,
            authorization_code_time_to_live: None,

            base: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H>>
    for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H> {
        &self.base
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseHttpConfigurer<OAuth2AuthorizationServerConfigurer<H>, H> {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn init(&mut self, http: &mut H) {
        let context = self.authorization_server_context(http);

        self.ignore_csrf_protection(http, &context);

        http.add_filter(AuthorizationServerMetadataEndpointFilter::new(
            context.clone(),
        ));
        http.add_filter(OAuth2ClientRegistrationEndpointFilter::new(context.clone()));
        http.add_filter(JwkSetEndpointFilter::new(context.clone()));
        http.add_filter(TokenIntrospectionEndpointFilter::new(context.clone()));
        http.add_filter(TokenRevocationEndpointFilter::new(context.clone()));
        http.add_filter(OAuth2PushedAuthorizationRequestEndpointFilter::new(
            context.clone(),
        ));
        http.add_filter(OAuth2DeviceAuthorizationEndpointFilter::new(
            context.clone(),
        ));
        http.add_filter(UserInfoEndpointFilter::new(context.clone()));
        http.add_filter(OAuth2DeviceVerificationEndpointFilter::new(context.clone()));
        http.add_filter(AuthorizationEndpointFilter::new(context.clone()));
        http.add_filter(TokenEndpointFilter::new(context.clone()));
        http.add_filter(LogoutEndpointFilter::new(context.clone()));
    }

    fn configure(&mut self, _http: &mut H) {}
}

impl<H> Deref for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for OAuth2AuthorizationServerConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

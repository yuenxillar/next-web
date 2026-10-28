use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

use next_web_core::{ApplicationContext, Required};

use crate::{
    authentication::AuthenticationProvider,
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{logout_configurer::LogoutConfigurer, BaseHttpConfigurer},
            HttpSecurityBuilder,
        },
    },
    oauth2::server::resource::jwt::{JwtDecoder, NimbusJwtDecoder},
    oauth2::{
        client::{
            oidc_back_channel_logout::{
                EitherLogoutHandler, OAuth2ClientConfigurerUtils,
                OidcBackChannelLogoutAuthenticationProvider, OidcBackChannelLogoutFilter,
                OidcBackChannelLogoutHandler, OidcLogoutAuthenticationConverter,
            },
            OidcSessionRegistry,
        },
        ClientRegistrationRepository,
    },
    web::{
        authentication::{
            logout::{CompositeLogoutHandler, LogoutHandler, SecurityContextLogoutHandler},
            AuthenticationConverter,
        },
        csrf::CsrfFilter,
        default_security_filter_chain::DefaultSecurityFilterChain,
    },
};

/// Adds OIDC Logout support, in particular OIDC Back-Channel Logout, to an
/// application. Mirrors Spring Security's `OidcLogoutConfigurer`.
#[derive(Clone)]
pub struct OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    client_registration_repository: Option<Arc<dyn ClientRegistrationRepository>>,
    oidc_session_registry: Option<Arc<dyn OidcSessionRegistry>>,
    back_channel: Option<BackChannelLogoutConfigurer>,
    base: BaseHttpConfigurer<Self, B>,
}

impl<B> OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    pub fn new(_context: Arc<RwLock<Box<dyn ApplicationContext>>>) -> Self {
        Self::default()
    }

    /// Sets the repository of client registrations.
    pub fn client_registration_repository(
        &mut self,
        client_registration_repository: Arc<dyn ClientRegistrationRepository>,
    ) -> &mut Self {
        self.client_registration_repository = Some(client_registration_repository);
        self
    }

    /// Sets the registry for managing the OIDC client-provider session link.
    pub fn oidc_session_registry(
        &mut self,
        oidc_session_registry: Arc<dyn OidcSessionRegistry>,
    ) -> &mut Self {
        self.oidc_session_registry = Some(oidc_session_registry);
        self
    }

    /// Configures OIDC Back-Channel Logout using the provided customizer.
    pub fn back_channel<F>(&mut self, customizer: F) -> &mut Self
    where
        F: FnOnce(&mut BackChannelLogoutConfigurer),
    {
        if self.back_channel.is_none() {
            self.back_channel = Some(BackChannelLogoutConfigurer::default());
        }
        if let Some(back_channel) = self.back_channel.as_mut() {
            customizer(back_channel);
        }
        self
    }
}

impl<B> Default for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn default() -> Self {
        Self {
            client_registration_repository: None,
            oidc_session_registry: None,
            back_channel: None,
            base: Default::default(),
        }
    }
}

impl<B> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>
    for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
    B: 'static,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        self.base.get_mut_object()
    }
}

impl<B> SecurityConfigurer<DefaultSecurityFilterChain, B> for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
    B: 'static,
{
    fn init(&mut self, http: &mut B) {
        if let Some(repository) = &self.client_registration_repository {
            http.set_shared_object(repository.clone());
        }
        if let Some(registry) = &self.oidc_session_registry {
            http.set_shared_object(registry.clone());
        }
    }

    fn configure(&mut self, http: &mut B) {
        if let Some(back_channel) = self.back_channel.as_mut() {
            back_channel.configure(http);
        }
    }
}

impl<B> Deref for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    type Target = BaseHttpConfigurer<Self, B>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<B> DerefMut for OidcLogoutConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Configurer for OIDC Back-Channel Logout. Mirrors the nested
/// `BackChannelLogoutConfigurer` of Spring Security's `OidcLogoutConfigurer`.
#[derive(Clone, Default)]
pub struct BackChannelLogoutConfigurer {
    authentication_converter: Option<Arc<dyn AuthenticationConverter>>,
    logout_handler: Option<Arc<dyn LogoutHandler>>,
    logout_uri: Option<String>,
    jwt_decoder: Option<Arc<dyn JwtDecoder>>,
}

impl BackChannelLogoutConfigurer {
    /// Sets the URI this application's back-channel logout endpoint listens on.
    pub fn logout_uri(&mut self, logout_uri: &str) -> &mut Self {
        self.logout_uri = Some(logout_uri.to_string());
        self
    }

    /// Sets the `LogoutHandler` used to terminate each individual provider
    /// session. Overrides any value supplied via `logout_uri`.
    pub fn logout_handler(&mut self, logout_handler: Arc<dyn LogoutHandler>) -> &mut Self {
        self.logout_handler = Some(logout_handler);
        self
    }

    /// Overrides the `AuthenticationConverter` used to extract the logout token.
    pub fn authentication_converter(
        &mut self,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) -> &mut Self {
        self.authentication_converter = Some(authentication_converter);
        self
    }

    /// Sets the decoder of the logout tokens the provider issues.
    ///
    /// A logout token is a JWT signed by the provider, so it has to be verified
    /// with the keys of that provider — the JWK set its registration names. Without
    /// a decoder the configuration refuses every logout token with 400, because it
    /// has no key to verify one with.
    pub fn jwt_decoder(&mut self, jwt_decoder: Arc<dyn JwtDecoder>) -> &mut Self {
        self.jwt_decoder = Some(jwt_decoder);
        self
    }

    fn resolve_logout_handler<B: HttpSecurityBuilder<B> + 'static>(
        &self,
        http: &mut B,
    ) -> Arc<dyn LogoutHandler> {
        if let Some(logout_uri) = &self.logout_uri {
            let registry = OAuth2ClientConfigurerUtils::get_oidc_session_registry(http);
            Arc::new(OidcBackChannelLogoutHandler::with_logout_uri(
                registry,
                logout_uri.clone(),
            ))
        } else if let Some(handler) = &self.logout_handler {
            handler.clone()
        } else {
            let registry = OAuth2ClientConfigurerUtils::get_oidc_session_registry(http);
            Arc::new(OidcBackChannelLogoutHandler::new(registry))
        }
    }

    fn resolve_session_logout<B: HttpSecurityBuilder<B> + 'static>(
        &self,
        http: &mut B,
    ) -> Arc<dyn LogoutHandler> {
        if let Some(logout) = http.configurer::<LogoutConfigurer<B>>() {
            let handlers = logout.get_logout_handlers().to_vec();
            Arc::new(CompositeLogoutHandler::new(handlers))
        } else {
            Arc::new(SecurityContextLogoutHandler::default())
        }
    }

    pub fn configure<B: HttpSecurityBuilder<B> + 'static>(&mut self, http: &mut B) {
        let oidc_logout = self.resolve_logout_handler(http);
        let session_logout = self.resolve_session_logout(http);
        let either = EitherLogoutHandler::new(oidc_logout, session_logout);

        let converter = self.authentication_converter.clone().unwrap_or_else(|| {
            let repository = OAuth2ClientConfigurerUtils::get_client_registration_repository(http);
            Arc::new(OidcLogoutAuthenticationConverter::new(repository))
        });

        let decoder: Arc<dyn JwtDecoder> = self
            .jwt_decoder
            .clone()
            .unwrap_or_else(|| Arc::new(NimbusJwtDecoder::default()));
        let provider: Arc<dyn AuthenticationProvider> =
            Arc::new(OidcBackChannelLogoutAuthenticationProvider::new(decoder));

        let filter = OidcBackChannelLogoutFilter::new(converter, provider, Arc::new(either));
        http.add_filter_before::<OidcBackChannelLogoutFilter, CsrfFilter>(filter);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        any::TypeId,
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use axum::{body::Body, extract::Request};
    use next_web_core::{
        async_trait,
        http::{HttpMethod, HttpRequest as _},
    };

    use super::*;
    use crate::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder,
            web::builders::HttpSecurity,
        },
        core::AuthenticationError,
        oauth2::{
            client::{
                InMemoryOidcSessionRegistry, OidcLogoutToken, OidcSessionRegistry,
            },
            server::resource::jwt::{Jwt, JwtDecoder},
            ClientRegistration, InMemoryClientRegistrationRepository,
        },
        web::{
            security_filter_chain::SecurityFilterChain, test_support::Chain,
            util::matcher::PathPatternRequestMatcherBuilder,
        },
    };

    /// The registration the logout tokens of the tests belong to.
    const REGISTRATION_ID: &str = "okta";

    /// Returns the registration the logout tokens of the tests belong to.
    fn registration() -> ClientRegistration {
        ClientRegistration::new(
            REGISTRATION_ID,
            "Okta",
            "https://idp.example.org/oauth2/authorize",
        )
        .set_client_id("client")
        .set_issuer_uri("https://idp.example.org")
    }

    /// Verifies the logout token of the tests, and refuses every other token.
    struct TestDecoder;

    #[async_trait]
    impl JwtDecoder for TestDecoder {
        async fn decode(&self, token: &str) -> Result<Jwt, AuthenticationError> {
            if token != "the-logout-token" {
                return Err(AuthenticationError::new(
                    "the token is not one this decoder accepts",
                ));
            }

            Ok(Jwt::new(token).with_claims(HashMap::from([
                ("iss".to_string(), "https://idp.example.org".to_string()),
                ("aud".to_string(), "client".to_string()),
                ("sub".to_string(), "Max".to_string()),
                ("sid".to_string(), "the-session".to_string()),
                ("jti".to_string(), "the-token-id".to_string()),
            ])))
        }
    }

    /// Records the logout tokens the chain accepted.
    #[derive(Clone, Default)]
    struct RecordingLogoutHandler {
        logout_tokens: Arc<Mutex<Vec<OidcLogoutToken>>>,
    }

    impl RecordingLogoutHandler {
        /// Returns the logout tokens the chain handed over.
        fn logout_tokens(&self) -> Vec<OidcLogoutToken> {
            self.logout_tokens
                .lock()
                .map(|tokens| tokens.clone())
                .unwrap_or_default()
        }
    }

    #[async_trait]
    impl LogoutHandler for RecordingLogoutHandler {
        async fn logout(
            &self,
            _request: &mut dyn next_web_core::http::HttpRequest,
            _response: &mut dyn next_web_core::http::HttpResponse,
            authentication: Option<&Arc<dyn crate::core::Authentication>>,
        ) {
            let logout_token = authentication
                .and_then(|authentication| {
                    (authentication.as_ref() as &dyn std::any::Any)
                        .downcast_ref::<crate::oauth2::client::OidcBackChannelLogoutAuthentication>()
                })
                .map(|authentication| authentication.logout_token().clone());
            if let (Some(logout_token), Ok(mut tokens)) = (logout_token, self.logout_tokens.lock()) {
                tokens.push(logout_token);
            }
        }
    }

    /// Returns an HTTP security whose back-channel logout is configured for one
    /// registration.
    fn http_security(logout_handler: RecordingLogoutHandler) -> HttpSecurity {
        let logout_handler = Arc::new(logout_handler);
        let mut shared_objects: HashMap<TypeId, Box<dyn next_web_core::any::AnyClone>> =
            HashMap::new();
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(PathPatternRequestMatcherBuilder::default()),
        );

        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        // The CSRF filter runs behind the logout one, so the chain of an
        // application that protects its forms has to be assembled here.
        http.csrf(|_| {});
        http.oidc_logout(|logout| {
            logout
                .client_registration_repository(Arc::new(
                    InMemoryClientRegistrationRepository::new(vec![registration()]),
                ))
                .oidc_session_registry(Arc::new(InMemoryOidcSessionRegistry::default()))
                .back_channel(|back_channel| {
                    back_channel
                        .jwt_decoder(Arc::new(TestDecoder))
                        .logout_handler(logout_handler.clone());
                });
        });

        http
    }

    /// Returns the names of the filters of a chain.
    fn filter_names(mut http: HttpSecurity) -> Vec<String> {
        http.build()
            .get_filters()
            .iter()
            .map(|filter| filter.name().to_owned())
            .collect()
    }

    /// Runs one request through the chain, and returns the status of the answer.
    async fn run(mut http: HttpSecurity, method: HttpMethod, uri: &str, form: Vec<u8>) -> usize {
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(next_web_core::http::RequestBody::new(form));
        let mut response = axum::response::Response::default();
        Chain::new(filters)
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        response.status().as_u16() as usize
    }

    #[test]
    fn the_configuration_installs_the_filter_before_the_csrf_filter() {
        let names = filter_names(http_security(RecordingLogoutHandler::default()));

        let logout = names
            .iter()
            .position(|name| name == "OidcBackChannelLogoutFilter")
            .unwrap_or_else(|| panic!("the logout filter is installed, the filters were: {names:?}"));
        let csrf = names
            .iter()
            .position(|name| name == "CsrfFilter")
            .unwrap_or_else(|| panic!("the CSRF filter is installed, the filters were: {names:?}"));
        assert!(
            logout < csrf,
            "a logout from the provider carries no CSRF token, so the filter runs before the CSRF one, the filters were: {names:?}"
        );
    }

    #[test]
    fn the_configuration_shares_the_repositories() {
        let mut http = http_security(RecordingLogoutHandler::default());
        let _chain = http.build();

        assert!(
            http.shared_object::<Arc<dyn ClientRegistrationRepository>>()
                .is_some(),
            "the registrations of the clients are shared"
        );
        assert!(
            http.shared_object::<Arc<dyn OidcSessionRegistry>>().is_some(),
            "the client-provider session registry is shared"
        );
    }

    #[tokio::test]
    async fn a_logout_token_ends_the_sessions_of_the_provider() {
        let logout_handler = RecordingLogoutHandler::default();
        let http = http_security(logout_handler.clone());

        let status = run(
            http,
            HttpMethod::POST,
            &format!("https://sp.example.org/logout/connect/back-channel/{REGISTRATION_ID}"),
            b"logout_token=the-logout-token".to_vec(),
        )
        .await;

        assert_eq!(status, 200, "the logout is accepted");
        let logout_tokens = logout_handler.logout_tokens();
        assert_eq!(
            logout_tokens.len(),
            1,
            "the logout token reaches the handler of the configuration"
        );
        assert_eq!(
            logout_tokens[0].subject(),
            Some("Max"),
            "the handler ends the sessions of the user the token names"
        );
        assert_eq!(
            logout_tokens[0].session_id(),
            Some("the-session"),
            "the handler knows the session the token ends"
        );
    }

    #[tokio::test]
    async fn a_request_that_is_not_a_logout_is_left_alone() {
        let logout_handler = RecordingLogoutHandler::default();
        let http = http_security(logout_handler.clone());

        let status = run(
            http,
            HttpMethod::GET,
            &format!("https://sp.example.org/logout/connect/back-channel/{REGISTRATION_ID}"),
            Vec::new(),
        )
        .await;

        assert_eq!(status, 200, "the request is left to the application");
        assert!(
            logout_handler.logout_tokens().is_empty(),
            "no logout is performed for a request that carries no logout token"
        );
    }

    #[tokio::test]
    async fn a_logout_token_that_cannot_be_verified_is_refused() {
        let logout_handler = RecordingLogoutHandler::default();
        let http = http_security(logout_handler.clone());

        let status = run(
            http,
            HttpMethod::POST,
            &format!("https://sp.example.org/logout/connect/back-channel/{REGISTRATION_ID}"),
            b"logout_token=another-token".to_vec(),
        )
        .await;

        assert_eq!(
            status, 400,
            "a logout token the decoder refuses is answered with 400"
        );
        assert!(
            logout_handler.logout_tokens().is_empty(),
            "a token that cannot be verified ends no session"
        );
    }
}

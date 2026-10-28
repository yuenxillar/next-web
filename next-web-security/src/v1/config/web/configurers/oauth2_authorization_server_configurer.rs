//! Configures the endpoints of the OAuth 2.0 authorization server.

use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::Required;

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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{body::Body, extract::Request, response::Response};
    use base64::{Engine, engine::general_purpose::STANDARD};

    use super::*;
    use crate::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder,
            web::builders::HttpSecurity,
        },
        core::context::SecurityContextHolder,
        oauth2::server::authorization::{
            AuthorizationGrantType, ClientAuthenticationMethod, ClientSettings,
            InMemoryRegisteredClientRepository, RegisteredClient,
        },
        web::{security_filter_chain::SecurityFilterChain, test_support::Chain},
    };

    /// The client of the tests.
    const CLIENT_ID: &str = "client";
    /// The secret of the client of the tests.
    const CLIENT_SECRET: &str = "secret";
    /// The redirect URI of the client of the tests.
    const REDIRECT_URI: &str = "https://client.example.org/callback";

    /// Returns the client of the tests.
    ///
    /// The client is one the tests drive by hand: it asks for no proof key and no
    /// consent, so the two endpoints can be walked without a browser.
    fn registered_client() -> RegisteredClient {
        RegisteredClient::with_client_id(CLIENT_ID)
            .set_client_secret(format!("{{noop}}{CLIENT_SECRET}"))
            .client_authentication_method(ClientAuthenticationMethod::ClientSecretBasic)
            .authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
            .redirect_uri(REDIRECT_URI)
            .scope("read")
            .set_client_settings(
                ClientSettings::builder()
                    .require_proof_key(false)
                    .require_authorization_consent(false)
                    .build(),
            )
    }

    /// Returns an HTTP security that serves the authorization server.
    fn http_security() -> HttpSecurity {
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
        http.csrf(|_| {});
        http.oauth2_authorization_server(|server| {
            server.registered_client_repository(Arc::new(
                InMemoryRegisteredClientRepository::new(vec![registered_client()]),
            ));
        });

        http
    }

    /// Returns a request of the given method and URI, with the given form body.
    fn request(method: &str, uri: &str, form: &str) -> Request {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(next_web_core::http::RequestBody::new(form.as_bytes().to_vec()));

        request
    }

    /// Returns the filters of the chain an application serves.
    fn filters_of(
        mut http: HttpSecurity,
    ) -> Arc<Vec<Arc<dyn next_web_core::http::filter::HttpFilter>>> {
        let chain = http.build();

        Arc::new(chain.get_filters().to_vec())
    }

    /// Runs one request through the chain, and returns the answer.
    async fn run(
        filters: &Arc<Vec<Arc<dyn next_web_core::http::filter::HttpFilter>>>,
        mut request: Request,
    ) -> Response {
        let mut response = Response::default();
        Chain::new(filters.clone())
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        response
    }

    /// Returns the `Location` header of a response.
    fn redirect_of(response: &Response) -> Option<String> {
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    /// Returns the body of a response as a string.
    async fn body_of(response: Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is collected");

        String::from_utf8_lossy(&body).to_string()
    }

    /// Signs the user in for the authorization endpoint.
    fn sign_in() {
        use crate::web::authentication::preauth::PreAuthenticatedAuthenticationToken;

        let authentication =
            PreAuthenticatedAuthenticationToken::new(Arc::new("Max".to_string()), None);
        SecurityContextHolder::get_context().set_authentication(Some(Arc::new(authentication)));
    }

    /// Returns the authorization code of the redirect, if the redirect carries one.
    fn code_of(location: &str) -> Option<String> {
        location
            .split(['?', '&'])
            .find_map(|parameter| parameter.strip_prefix("code="))
            .map(str::to_owned)
    }

    /// Returns the state of the redirect, if it carries one.
    fn state_of(location: &str) -> Option<String> {
        location
            .split(['?', '&'])
            .find_map(|parameter| parameter.strip_prefix("state="))
            .map(str::to_owned)
    }

    /// Returns the `Authorization` header of a request that authenticates with
    /// the credentials of the client of the tests.
    fn client_authorization() -> String {
        let credentials = STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"));

        format!("Basic {credentials}")
    }

    #[test]
    fn the_configuration_installs_the_endpoints() {
        let mut http = http_security();
        let names = http
            .build()
            .get_filters()
            .iter()
            .map(|filter| filter.name().to_owned())
            .collect::<Vec<_>>();

        for endpoint in [
            "AuthorizationEndpointFilter",
            "TokenEndpointFilter",
            "AuthorizationServerMetadataEndpointFilter",
            "JwkSetEndpointFilter",
            "TokenIntrospectionEndpointFilter",
            "TokenRevocationEndpointFilter",
            "UserInfoEndpointFilter",
        ] {
            assert!(
                names.iter().any(|name| name == endpoint),
                "{endpoint} is installed, the filters were: {names:?}"
            );
        }
    }

    #[tokio::test]
    async fn an_authorization_request_is_answered_with_a_code() {
        sign_in();
        let filters = filters_of(http_security());
        let request = request(
            "GET",
            &format!(
                "https://auth.example.org/oauth2/authorize?response_type=code&client_id={CLIENT_ID}&redirect_uri={REDIRECT_URI}&scope=read&state=the-state"
            ),
            "",
        );

        let response = run(&filters, request).await;
        let location = redirect_of(&response).expect("the authorization endpoint redirects");

        assert_eq!(
            response.status().as_u16(),
            302,
            "the request is answered with a redirect, it was: {location}"
        );
        assert!(
            location.starts_with(REDIRECT_URI),
            "the redirect goes to the client, it was: {location}"
        );
        assert!(
            code_of(&location).is_some(),
            "the redirect carries an authorization code, it was: {location}"
        );
        assert_eq!(
            state_of(&location).as_deref(),
            Some("the-state"),
            "the state of the request is returned to the client, it was: {location}"
        );
    }

    #[tokio::test]
    async fn the_code_is_exchanged_for_a_token() {
        sign_in();
        // One chain serves both halves of the grant: the code is stored in the
        // authorization service of the chain the code was issued by.
        let filters = filters_of(http_security());
        let response = run(
            &filters,
            request(
                "GET",
                &format!(
                    "https://auth.example.org/oauth2/authorize?response_type=code&client_id={CLIENT_ID}&redirect_uri={REDIRECT_URI}&scope=read&state=the-state"
                ),
                "",
            ),
        )
        .await;
        let location = redirect_of(&response).expect("the authorization endpoint redirects");
        let code = code_of(&location).expect("the redirect carries a code");

        // The second half: the client exchanges the code for a token.
        let mut token_request = request(
            "POST",
            "https://auth.example.org/oauth2/token",
            &format!("grant_type=authorization_code&code={code}&redirect_uri={REDIRECT_URI}"),
        );
        token_request.headers_mut().insert(
            "authorization",
            client_authorization().parse().expect("the header is valid"),
        );
        let token_response = run(&filters, token_request).await;
        let status = token_response.status().as_u16();
        let body = body_of(token_response).await;

        assert_eq!(
            status, 200,
            "the code is exchanged for a token, the answer was: {body}"
        );
        assert!(
            body.contains("\"access_token\""),
            "the answer carries an access token, it was: {body}"
        );
        assert!(
            body.contains("\"token_type\""),
            "the answer names the type of the token, it was: {body}"
        );
    }

    #[tokio::test]
    async fn a_request_without_a_registered_client_is_refused() {
        sign_in();
        let response = run(
            &filters_of(http_security()),
            request(
                "GET",
                "https://auth.example.org/oauth2/authorize?response_type=code&client_id=another-client&redirect_uri=https%3A%2F%2Fclient.example.org%2Fcallback&scope=read",
                "",
            ),
        )
        .await;
        let location = redirect_of(&response);
        let body = body_of(response).await;

        assert!(
            location.is_none(),
            "a request of an unknown client is not redirected, it was: {body}"
        );
        assert!(
            body.contains("invalid_request"),
            "the answer names the error, it was: {body}"
        );
    }

    #[tokio::test]
    async fn the_metadata_of_the_server_is_published() {
        let response = run(
            &filters_of(http_security()),
            request(
                "GET",
                "https://auth.example.org/.well-known/oauth-authorization-server",
                "",
            ),
        )
        .await;
        let status = response.status().as_u16();
        let body = body_of(response).await;

        assert_eq!(status, 200, "the metadata is published, it was: {body}");
        assert!(
            body.contains("authorization_endpoint"),
            "the metadata names the endpoints of the server, it was: {body}"
        );
    }
}

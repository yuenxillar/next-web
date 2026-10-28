//! The chain every configuration of the builder composes.
//!
//! A filter of an application is only reached when the chain holds it in the right
//! place: a login filter that runs after the filter that authorizes the request
//! never sees the request, and a filter that runs before the one that prepares the
//! session never sees the session. The tests here build one chain that holds the
//! filters of every configuration of [`HttpSecurity`], and hold the order of the
//! chain against the order of the reference implementation.
//!
//! They also hold that the chain *runs*: a request that the application permits
//! reaches the end of the chain, and a request it does not permit is stopped by the
//! filter that authorizes it.

// The test drives the Spring Security style tree.
#![cfg(feature = "v1")]

use std::{collections::HashMap, sync::Arc};

use axum::{body::Body, extract::Request, response::Response};
use next_web_core::{
    any::AnyClone,
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
    web::cors::{CorsConfiguration, CorsConfigurationSource},
    Named,
};
use next_web_security::{
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        http::SessionCreationPolicy,
        security_builder::SecurityBuilder,
        web::{builders::HttpSecurity, configurers::DefaultLoginPageConfigurer},
    },
    core::userdetails::{User, UserDetailsService},
    provisioning::InMemoryUserDetailsManager,
    web::{
        security_filter_chain::SecurityFilterChain, session::ForceEagerSessionCreationFilter,
        util::matcher::PathPatternRequestMatcherBuilder,
    },
};

#[cfg(feature = "saml2")]
use next_web_security::saml2::registration::{
    InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
};

/// A source of CORS configurations that configures none.
struct NoCors;

impl CorsConfigurationSource for NoCors {
    fn cors_configuration(&self, _request: &dyn HttpRequest) -> Option<&CorsConfiguration> {
        None
    }
}

/// A handler that delivers a one-time token; the chain of the tests only holds it.
struct OttSuccessHandler;

impl next_web_security::web::authentication::ott::OneTimeTokenGenerationSuccessHandler
    for OttSuccessHandler
{
    fn handle(
        &self,
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _ott: &dyn next_web_security::authentication::ott::OneTimeToken,
    ) {
    }
}

/// A decoder that verifies no token; the chain of the tests only holds it.
#[cfg(feature = "oauth2-resource-server")]
struct StubDecoder;

#[cfg(feature = "oauth2-resource-server")]
#[async_trait]
impl next_web_security::oauth2::server::resource::jwt::JwtDecoder for StubDecoder {
    async fn decode(
        &self,
        token: &str,
    ) -> Result<
        next_web_security::oauth2::server::resource::jwt::Jwt,
        next_web_security::core::AuthenticationError,
    > {
        Err(next_web_security::core::AuthenticationError::new(format!(
            "the decoder of the tests refuses {token}"
        )))
    }
}

/// The terminal of the chains of the tests: the application.
#[derive(Clone)]
struct Terminal;

#[async_trait]
impl HttpFilter for Terminal {
    async fn do_filter(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        response.set_body(b"reached the application".to_vec());

        Ok(())
    }

    fn supports(&self, name: &str) -> bool {
        name == "Terminal"
    }
}

impl Named for Terminal {
    fn name(&self) -> &str {
        "Terminal"
    }
}

/// Runs the filters of a chain in order, the way the chain proxy does.
#[derive(Clone)]
struct Chain {
    filters: Arc<Vec<Arc<dyn HttpFilter>>>,
    index: usize,
}

impl Chain {
    /// Returns a chain that starts at the first filter.
    fn new(filters: Arc<Vec<Arc<dyn HttpFilter>>>) -> Self {
        Self { filters, index: 0 }
    }

    /// Runs the filter at the current index and everything after it.
    async fn run(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        HttpFilterChain::do_filter(self, request, response).await
    }
}

#[async_trait]
impl HttpFilterChain for Chain {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let Some(filter) = self.filters.get(self.index) else {
            return Ok(());
        };
        let next = Self {
            filters: self.filters.clone(),
            index: self.index + 1,
        };

        filter.do_filter(request, response, &next).await
    }
}

/// Returns the user details service of the configurations.
fn user_details_service() -> Arc<dyn UserDetailsService> {
    Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
        User::with_username("Max")
            .password(Some("password".to_string()))
            .build(),
    )]))
}

/// Returns the registrations of the SAML 2.0 configurations.
#[cfg(feature = "saml2")]
fn relying_party_registrations() -> Arc<InMemoryRelyingPartyRegistrationRepository> {
    Arc::new(InMemoryRelyingPartyRegistrationRepository::new(vec![
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("https://idp.example.org")
            .single_sign_on_service_location("https://idp.example.org/sso")
            .single_logout_service_location("https://sp.example.org/logout/saml2/slo")
            .single_logout_service_response_location("https://idp.example.org/slo")
            .build(),
    ]))
}

/// Returns the registrations of the OAuth 2.0 client of the tests.
#[cfg(feature = "oauth2-client")]
fn client_registrations() -> Arc<dyn next_web_security::oauth2::ClientRegistrationRepository> {
    Arc::new(
        next_web_security::oauth2::InMemoryClientRegistrationRepository::new(vec![
            next_web_security::oauth2::ClientRegistration::new(
                "test",
                "Test",
                "https://idp.example.org/authorize",
            )
            .set_client_id("client")
            .set_client_secret("secret")
            .set_redirect_uri("https://sp.example.org/login/oauth2/code/test")
            .set_token_uri("https://idp.example.org/token")
            .set_scopes(["read"]),
        ]),
    )
}

/// Returns the HTTP security every configuration of the builder is applied to.
///
/// The configurations are the ones of an application that offers every login
/// method the builder has: the chain they compose is what the test holds in order.
fn maximal() -> HttpSecurity {
    let mut shared_objects: HashMap<std::any::TypeId, Box<dyn AnyClone>> = HashMap::new();
    shared_objects.insert(
        std::any::TypeId::of::<PathPatternRequestMatcherBuilder>(),
        Box::new(PathPatternRequestMatcherBuilder::default()),
    );
    let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);

    http.csrf(|_| {})
        .headers(|_| {})
        .cors(|cors| {
            cors.configuration_source(Arc::new(NoCors));
        })
        .session_management(|session| {
            session
                .session_creation_policy(SessionCreationPolicy::Always)
                .maximum_sessions(1);
        })
        .security_context(|_| {})
        .request_cache(|_| {})
        .anonymous(|_| {})
        .error_handling(|_| {})
        .form_login(|_| {})
        .http_basic(|_| {})
        .logout(|_| {})
        .remember_me(|remember| {
            remember
                .user_details_service(user_details_service())
                .key("the-key");
        })
        .one_time_token_login(|one_time| {
            one_time
                .user_details_service(user_details_service())
                .token_generation_success_handler(Arc::new(OttSuccessHandler));
        })
        .password_management(|_| {})
        .x509(|x509| {
            // The X.509 configuration takes the service itself, not an erased one.
            x509.user_details_service(InMemoryUserDetailsManager::with_users(vec![Arc::new(
                User::with_username("Max")
                    .password(Some("password".to_string()))
                    .build(),
            )]));
        });

    #[cfg(feature = "webauthn")]
    http.web_authn(|web_authn| {
        web_authn
            .rp_id("localhost")
            .user_details_service(user_details_service());
    });

    #[cfg(feature = "saml2")]
    http.saml2_login(|saml2| {
        saml2
            .relying_party_registration_repository(relying_party_registrations())
            .user_details_service(user_details_service());
    })
    .saml2_metadata(|metadata| {
        metadata.relying_party_registration_repository(relying_party_registrations());
    })
    .saml2_logout(|logout| {
        logout.relying_party_registration_repository(relying_party_registrations());
    });

    http.authorize_http_requests(|authorize| {
        authorize.any_request().permit_all();
    });
    http.with(DefaultLoginPageConfigurer::default(), |_| {});

    #[cfg(feature = "oauth2-client")]
    {
        http.oauth2_login(|login| {
            login.client_registration_repository(client_registrations());
        });
        http.oauth2_client(|client| {
            client.client_registration_repository(client_registrations());
        });
    }
    #[cfg(feature = "oauth2-resource-server")]
    {
        http.oauth2_resource_server(|resource_server| {
            resource_server.jwt(|jwt| {
                jwt.decoder(Arc::new(StubDecoder));
            });
        });
    }
    #[cfg(feature = "oauth2-authorization-server")]
    {
        http.oauth2_authorization_server(|server| {
            server.registered_client_repository(Arc::new(
                next_web_security::oauth2::server::authorization::InMemoryRegisteredClientRepository::default(),
            ));
        });
    }
    #[cfg(all(feature = "oauth2-client", feature = "oauth2-resource-server"))]
    {
        http.oidc_logout(|logout| {
            logout
                .client_registration_repository(client_registrations())
                .back_channel(|back_channel| {
                    back_channel.jwt_decoder(Arc::new(StubDecoder));
                });
        });
    }

    http
}

/// Returns the names of the filters of a chain, in the order they run.
fn filter_names(mut http: HttpSecurity) -> Vec<String> {
    http.build()
        .get_filters()
        .iter()
        .map(|filter| filter.name().to_owned())
        .collect()
}

/// Asserts that the filter that comes first in the chain runs before the second
/// one.
fn assert_before(names: &[String], first: &str, second: &str) {
    let position = |name: &str| {
        names
            .iter()
            .position(|filter| filter == name)
            .unwrap_or_else(|| panic!("{name} is part of the chain, the chain was: {names:?}"))
    };

    assert!(
        position(first) < position(second),
        "{first} runs before {second}, the chain was: {names:?}"
    );
}

/// Returns a request of the application.
fn request(method: &str, path: &str) -> Request {
    Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("a valid request")
}

#[test]
fn every_configuration_composes_into_one_ordered_chain() {
    let names = filter_names(maximal());

    // The session the application creates eagerly is created before anything reads
    // the request, and the context of the request is read before anything
    // authenticates it.
    assert_before(
        &names,
        "ForceEagerSessionCreationFilter",
        "SecurityContextHolderFilter",
    );
    assert_before(&names, "SecurityContextHolderFilter", "HeaderWriterFilter");
    assert_before(&names, "HeaderWriterFilter", "CorsFilter");
    assert_before(&names, "CorsFilter", "CsrfFilter");
    // The two logouts of an asserting party answer a signed message, so they run
    // before the filter that demands a CSRF token.
    #[cfg(feature = "saml2")]
    {
        assert_before(&names, "Saml2LogoutRequestFilter", "CsrfFilter");
        assert_before(&names, "Saml2LogoutResponseFilter", "CsrfFilter");
    }
    assert_before(&names, "CsrfFilter", "LogoutFilter");
    // The endpoints of an authorization server are served before the filters that
    // authenticate a resource owner, so the credentials of a client are not read as
    // the credentials of a user.
    #[cfg(feature = "oauth2-client")]
    {
        assert_before(
            &names,
            "LogoutFilter",
            "OAuth2AuthorizationRequestRedirectFilter",
        );
        assert_before(
            &names,
            "OAuth2AuthorizationRequestRedirectFilter",
            "GenerateOneTimeTokenFilter",
        );
        assert_before(
            &names,
            "OAuth2AuthorizationCodeGrantFilter",
            "GenerateOneTimeTokenFilter",
        );
        assert_before(
            &names,
            "X509AuthenticationFilter",
            "OAuth2LoginAuthenticationFilter",
        );
    }
    // The certificate of the connection is written into the request before the
    // filter that authenticates with it reads it.
    assert_before(
        &names,
        "ClientCertificateAttributeFilter",
        "X509AuthenticationFilter",
    );
    assert_before(
        &names,
        "X509AuthenticationFilter",
        "UsernamePasswordAuthenticationFilter",
    );
    assert_before(
        &names,
        "UsernamePasswordAuthenticationFilter",
        "DefaultResourcesFilter",
    );
    // The passkey ceremonies run after the resources they are served from, and
    // before the filters that authenticate a resource owner the other ways.
    #[cfg(feature = "webauthn")]
    {
        assert_before(
            &names,
            "DefaultResourcesFilter",
            "WebAuthnRegistrationFilter",
        );
        assert_before(
            &names,
            "WebAuthnRegistrationFilter",
            "WebAuthnAuthenticationFilter",
        );
    }
    #[cfg(all(feature = "webauthn", feature = "saml2"))]
    assert_before(
        &names,
        "WebAuthnAuthenticationFilter",
        "Saml2AuthenticationFilter",
    );
    #[cfg(feature = "saml2")]
    assert_before(
        &names,
        "Saml2AuthenticationFilter",
        "BasicAuthenticationFilter",
    );
    #[cfg(feature = "webauthn")]
    assert_before(
        &names,
        "WebAuthnAuthenticationFilter",
        "BasicAuthenticationFilter",
    );
    assert_before(
        &names,
        "DefaultResourcesFilter",
        "BasicAuthenticationFilter",
    );
    assert_before(
        &names,
        "BasicAuthenticationFilter",
        "RequestCacheAwareFilter",
    );
    assert_before(
        &names,
        "RequestCacheAwareFilter",
        "RememberMeAuthenticationFilter",
    );
    assert_before(
        &names,
        "RememberMeAuthenticationFilter",
        "AnonymousAuthenticationFilter",
    );
    assert_before(
        &names,
        "AnonymousAuthenticationFilter",
        "SessionManagementFilter",
    );
    // The authorization endpoint of the server runs after the error translation so
    // that a resource owner who is not logged in is sent to the login page, and
    // before the filter that authorizes the request so that it serves its own.
    assert_before(&names, "SessionManagementFilter", "ErrorTranslationFilter");
    assert_before(&names, "ErrorTranslationFilter", "AuthorizationFilter");

    // The routes a client and a resource owner use of an authorization server run
    // after the filters that prepare a request and before the ones that
    // authenticate a resource owner.
    #[cfg(feature = "oauth2-authorization-server")]
    {
        assert_before(&names, "LogoutFilter", "TokenEndpointFilter");
        assert_before(&names, "TokenEndpointFilter", "GenerateOneTimeTokenFilter");
        // A resource owner who is not logged in is sent to the login page by the
        // error translation filter, so the authorization endpoint runs after it.
        assert_before(
            &names,
            "ErrorTranslationFilter",
            "AuthorizationEndpointFilter",
        );
        assert_before(&names, "AuthorizationEndpointFilter", "AuthorizationFilter");
    }

    // Every filter an application can expect is part of the chain.
    #[allow(unused_mut)]
    let mut expected_filters = vec![
        "ForceEagerSessionCreationFilter",
        "SecurityContextHolderFilter",
        "HeaderWriterFilter",
        "CorsFilter",
        "CsrfFilter",
        "LogoutFilter",
        "GenerateOneTimeTokenFilter",
        "ClientCertificateAttributeFilter",
        "X509AuthenticationFilter",
        "RequestMatcherRedirectFilter",
        "UsernamePasswordAuthenticationFilter",
        "OneTimeTokenAuthenticationFilter",
        "DefaultResourcesFilter",
        "DefaultOneTimeTokenSubmitPageGeneratingFilter",
        "ConcurrentSessionFilter",
        "BasicAuthenticationFilter",
        "RequestCacheAwareFilter",
        "RememberMeAuthenticationFilter",
        "AnonymousAuthenticationFilter",
        "SessionManagementFilter",
        "ErrorTranslationFilter",
        "AuthorizationFilter",
    ];
    #[cfg(feature = "webauthn")]
    {
        expected_filters.extend([
            "DefaultWebAuthnRegistrationPageGeneratingFilter",
            "WebAuthnRegistrationFilter",
            "WebAuthnAuthenticationFilter",
        ]);
    }
    #[cfg(feature = "saml2")]
    {
        expected_filters.extend([
            "Saml2LogoutRequestFilter",
            "Saml2LogoutResponseFilter",
            "Saml2AuthenticationRequestFilter",
            "Saml2AuthenticationFilter",
            "Saml2MetadataFilter",
        ]);
    }
    #[cfg(feature = "oauth2-client")]
    {
        expected_filters.extend([
            "OAuth2AuthorizationRequestRedirectFilter",
            "OAuth2AuthorizationCodeGrantFilter",
            "OAuth2LoginAuthenticationFilter",
        ]);
    }
    #[cfg(feature = "oauth2-resource-server")]
    {
        expected_filters.push("BearerTokenAuthenticationFilter");
    }
    #[cfg(feature = "oauth2-authorization-server")]
    {
        expected_filters.extend([
            "OAuth2ClientRegistrationEndpointFilter",
            "AuthorizationServerMetadataEndpointFilter",
            "JwkSetEndpointFilter",
            "TokenIntrospectionEndpointFilter",
            "TokenRevocationEndpointFilter",
            "OAuth2PushedAuthorizationRequestEndpointFilter",
            "OAuth2DeviceAuthorizationEndpointFilter",
            "TokenEndpointFilter",
            "UserInfoEndpointFilter",
            "OAuth2DeviceVerificationEndpointFilter",
            "AuthorizationEndpointFilter",
            "LogoutEndpointFilter",
        ]);
    }
    #[cfg(all(feature = "oauth2-client", feature = "oauth2-resource-server"))]
    {
        expected_filters.push("OidcBackChannelLogoutFilter");
    }

    // Nothing a configuration contributes is dropped: the chain holds at least the
    // filters the features of this build expect.
    let expected_count = expected_filters.len();

    for expected in expected_filters {
        assert!(
            names.iter().any(|name| name == expected),
            "{expected} is part of the chain, the chain was: {names:?}"
        );
    }

    // The filters are ordered by the position they were registered with, so the
    // chain is sorted: a filter that was added without a position would panic while
    // the chain is built, which the call above already proved it does not.
    assert!(
        names.len() >= expected_count,
        "the chain of every configuration holds the filters of all of them, it was: {names:?}"
    );
}

#[test]
fn the_session_creation_policy_of_always_has_a_filter_of_its_own() {
    let names = filter_names(maximal());

    let position = names
        .iter()
        .position(|name| name == "ForceEagerSessionCreationFilter")
        .expect("a chain that creates its session eagerly holds the filter");
    assert_eq!(
        position, 0,
        "the filter runs first, the chain was: {names:?}"
    );
    let filter = ForceEagerSessionCreationFilter::default();
    assert_eq!(filter.name(), names[position]);
}

#[tokio::test]
async fn a_request_the_application_permits_reaches_it() {
    let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
    http.csrf(|_| {})
        .security_context(|_| {})
        .anonymous(|_| {})
        .error_handling(|_| {})
        .authorize_http_requests(|authorize| {
            authorize.any_request().permit_all();
        });
    let chain = http.build();
    let mut filters = chain.get_filters().to_vec();
    filters.push(Arc::new(Terminal) as Arc<dyn HttpFilter>);
    let mut request = request("GET", "/");
    let mut response = Response::default();
    Chain::new(Arc::new(filters))
        .run(&mut request, &mut response)
        .await
        .expect("the chain runs");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the body is collected");

    assert_eq!(
        String::from_utf8_lossy(&body),
        "reached the application",
        "a permitted request runs through every filter of the chain"
    );
}

#[tokio::test]
async fn a_request_the_application_does_not_permit_is_stopped() {
    let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
    http.csrf(|_| {})
        .security_context(|_| {})
        .anonymous(|_| {})
        .error_handling(|_| {})
        .authorize_http_requests(|authorize| {
            authorize.any_request().authenticated();
        });
    let chain = http.build();
    let mut filters = chain.get_filters().to_vec();
    filters.push(Arc::new(Terminal) as Arc<dyn HttpFilter>);
    let mut request = request("GET", "/");
    let mut response = Response::default();
    Chain::new(Arc::new(filters))
        .run(&mut request, &mut response)
        .await
        .expect("the chain runs");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the body is collected");

    assert_ne!(
        String::from_utf8_lossy(&body),
        "reached the application",
        "a request that is not permitted is stopped by the filter that authorizes it"
    );
    assert!(
        status.is_redirection() || status.is_client_error(),
        "a stopped request is answered with a redirect or a refusal, status was {status}"
    );
}

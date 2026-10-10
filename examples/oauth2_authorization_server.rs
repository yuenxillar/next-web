//! Example of an application that serves the OAuth 2.0 authorization server.
//!
//! The application serves the endpoints of the authorization server, the
//! resource server that accepts the tokens it issues, and the login of the
//! resource owner:
//!
//! * `GET /oauth2/authorize` grants an authorization code to a client after the
//!   resource owner has logged in,
//! * `POST /oauth2/token` exchanges the code, a refresh token, or the
//!   credentials of a client for an access token,
//! * `POST /oauth2/introspect` reports the state of a token, and
//!   `POST /oauth2/revoke` revokes one,
//! * `GET /oauth2/jwks` and `GET /.well-known/oauth-authorization-server`
//!   publish the keys and the metadata of the server,
//! * `GET /api/me` is a resource of the application, which the resource server
//!   protects with the bearer token the server issued,
//! * `GET /login` and `POST /login` log the resource owner in.
//!
//! The demo client is registered with the id `demo-client` and the secret
//! `secret`, and it redirects to `http://127.0.0.1:11000/app/callback`. The
//! resource owner is `user` with the password `password`, and signs in with
//! HTTP basic authentication.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin oauth2_authorization_server --features oauth2
//! ```
//!
//! The flow of the authorization code grant with `curl` is:
//!
//! ```text
//! # The resource owner logs in and grants the code.
//! curl -i -u user:password \
//!   "http://127.0.0.1:11000/app/oauth2/authorize?response_type=code&client_id=demo-client&redirect_uri=http%3A%2F%2F127.0.0.1%3A11000%2Fapp%2Fcallback&scope=read&state=xyz"
//!
//! # The client exchanges the code for a token.
//! curl -i -u demo-client:secret -X POST \
//!   -d "grant_type=authorization_code&code=<code>&redirect_uri=http%3A%2F%2F127.0.0.1%3A11000%2Fapp%2Fcallback" \
//!   http://127.0.0.1:11000/app/oauth2/token
//!
//! # The resource server accepts the token.
//! curl -i -H "Authorization: Bearer <access_token>" http://127.0.0.1:11000/app/api/me
//! ```

use std::sync::Arc;

use axum::routing::get;
use next_web::{
    core::{traits::apply_router::ApplyRouter, ApplicationContext, Ordered},
    macros::{autoconfigure::configuration, bind::singleton},
    Application, NextWebApplication,
};
use next_web_context::{ApplicationEvent, ApplicationEventPublisher};
use next_web_core::error::BoxError;
use next_web_security::{
    authentication::dao::DaoAuthenticationProvider,
    config::web::{builders::HttpSecurity, HttpSecurityBuilder, WebSecurityConfigurer},
    core::userdetails::User,
    crypto::bcrypt::BCryptPasswordEncoder,
    crypto::password::PasswordEncoder,
    oauth2::server::{
        authorization::{
            AuthorizationGrantType, AuthorizationServerSettings, ClientAuthenticationMethod,
            ClientSettings, InMemoryRegisteredClientRepository, JwkSource, RegisteredClient,
            SecretJwkSource,
        },
        resource::jwt::NimbusJwtDecoder,
    },
    provisioning::InMemoryUserDetailsManager,
};

/// The identifier of the demo client.
pub const DEMO_CLIENT_ID: &str = "demo-client";

/// The secret of the demo client.
pub const DEMO_CLIENT_SECRET: &str = "secret";

/// The redirect URI of the demo client.
pub const DEMO_REDIRECT_URI: &str = "http://127.0.0.1:11000/app/callback";

/// The issuer of the authorization server.
pub const DEMO_ISSUER: &str = "http://127.0.0.1:11000/app";

/// The secret the demo tokens are signed with.
pub const DEMO_SIGNING_SECRET: &str = "the-demo-secret-of-the-authorization-server";

/// The application of the example.
#[derive(Default)]
struct OAuth2AuthorizationServerApplication;

impl Application for OAuth2AuthorizationServerApplication {}

/// Contributes the event publisher the security configuration resolves.
#[derive(Clone)]
struct OAuth2AuthorizationServerAutoConfig;

#[configuration]
impl OAuth2AuthorizationServerAutoConfig {
    #[provider(name = "applicationEventPublisher")]
    fn application_event_publisher() -> Arc<dyn ApplicationEventPublisher> {
        #[derive(Clone)]
        struct NoopApplicationEventPublisher;

        impl ApplicationEventPublisher for NoopApplicationEventPublisher {
            fn publish_event(&self, _event: Box<dyn ApplicationEvent>) -> Result<(), BoxError> {
                Ok(())
            }
        }

        Arc::new(NoopApplicationEventPublisher)
    }
}

/// Contributes the router of the example.
#[singleton(binds = [Self::into_apply_router])]
#[derive(Clone)]
pub struct OAuth2AuthorizationServerRouter;

impl OAuth2AuthorizationServerRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl Ordered for OAuth2AuthorizationServerRouter {
    fn order(&self) -> i32 {
        0
    }
}

impl ApplyRouter for OAuth2AuthorizationServerRouter {
    /// Builds the router of the example.
    ///
    /// The client of the demonstration is redirected to `/callback` with the
    /// authorization code, which the route answers with.
    ///
    /// * `_ctx` - The context of the application.
    fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/public/hello", get(|| async { "public hello" }))
            .route("/api/me", get(|| async { "authenticated hello" }))
            .route(
                "/callback",
                get(|query: axum::extract::Query<std::collections::HashMap<String, String>>| async move {
                    format!(
                        "authorization code: {}",
                        query.get("code").map(String::as_str).unwrap_or("missing")
                    )
                }),
            )
    }
}

/// Returns the clients that are registered with the authorization server.
///
/// # Returns
///
/// The demo client, and a public client that has to use a proof key.
pub fn demo_registered_clients() -> Vec<RegisteredClient> {
    let demo_client = RegisteredClient::with_client_id(DEMO_CLIENT_ID)
        .set_client_secret(format!("{{noop}}{DEMO_CLIENT_SECRET}"))
        .set_client_name("Demo Client")
        .client_authentication_method(ClientAuthenticationMethod::ClientSecretBasic)
        .client_authentication_method(ClientAuthenticationMethod::ClientSecretPost)
        .authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
        .authorization_grant_type(AuthorizationGrantType::RefreshToken)
        .authorization_grant_type(AuthorizationGrantType::ClientCredentials)
        .redirect_uri(DEMO_REDIRECT_URI)
        .scope("read")
        .set_client_settings(ClientSettings::builder().require_proof_key(false).build());

    let public_client = RegisteredClient::with_client_id("public-client")
        .set_client_name("Public Client")
        .client_authentication_method(ClientAuthenticationMethod::None)
        .authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
        .redirect_uri(DEMO_REDIRECT_URI)
        .scope("read")
        .set_client_settings(ClientSettings::builder().require_proof_key(true).build());

    vec![demo_client, public_client]
}

/// Returns the settings of the authorization server of the example.
pub fn demo_authorization_server_settings() -> AuthorizationServerSettings {
    AuthorizationServerSettings::builder()
        .issuer(DEMO_ISSUER)
        .scopes(["read"])
        .build()
}

/// Returns the keys the tokens of the example are signed with.
pub fn demo_jwk_source() -> Arc<dyn JwkSource> {
    Arc::new(SecretJwkSource::new(DEMO_SIGNING_SECRET))
}

/// Returns the resource owner the example logs in.
///
/// # Returns
///
/// The user details service of the example, whose user is `user` with the
/// password `password`.
pub fn demo_user_details_service(
) -> Arc<dyn next_web_security::core::userdetails::UserDetailsService> {
    let password = BCryptPasswordEncoder::default()
        .encode(Some("password"))
        .expect("the password of the demo user is encoded")
        .expect("the encoded password exists");

    Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
        User::with_username("user")
            .password(Some(password))
            .roles(&["USER".to_string()])
            .build(),
    )]))
}

/// Configures the authorization server, the resource server, and the login of
/// the example.
#[singleton(binds = [Self::into_web_security_configure])]
#[derive(Clone)]
struct OAuth2AuthorizationServerWebSecurityConfigurer;

impl OAuth2AuthorizationServerWebSecurityConfigurer {
    fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigurer> {
        Box::new(self)
    }
}

impl WebSecurityConfigurer for OAuth2AuthorizationServerWebSecurityConfigurer {
    fn configure(&mut self, http: &mut HttpSecurity) {
        configure_oauth2_authorization_server(http);
    }
}

/// Applies the configuration of the example.
///
/// The application and the tests share the function, so the tests cover the
/// configuration the example ships.
///
/// # Arguments
///
/// * `http` - The HTTP security the configuration is applied to.
pub fn configure_oauth2_authorization_server(http: &mut HttpSecurity) {
    // The resource owner signs in with HTTP basic authentication, which is what
    // the authorization endpoint reads to find the authenticated resource
    // owner.
    let mut provider = DaoAuthenticationProvider::new(demo_user_details_service());
    provider.set_password_encoder(Arc::new(BCryptPasswordEncoder::default()));
    http.authentication_provider(Arc::new(provider));

    http.authorize_http_requests(|authorize| {
        authorize
            .request_matchers(&[
                "/public/**",
                "/callback",
                AuthorizationServerSettings::DEFAULT_AUTHORIZATION_SERVER_METADATA_ENDPOINT,
                AuthorizationServerSettings::DEFAULT_JWK_SET_ENDPOINT,
            ])
            .permit_all()
            .any_request()
            .authenticated();
    })
    .http_basic(|_| {})
    .oauth2_authorization_server(|authorization_server| {
        authorization_server
            .registered_client_repository(Arc::new(InMemoryRegisteredClientRepository::new(
                demo_registered_clients(),
            )))
            .authorization_server_settings(demo_authorization_server_settings())
            .jwk_source(demo_jwk_source());
    })
    .oauth2_resource_server(|resource_server| {
        resource_server.jwt(|jwt| {
            jwt.decoder(Arc::new(NimbusJwtDecoder::default()));
        });
    });
}

#[tokio::main]
async fn main() {
    NextWebApplication::<OAuth2AuthorizationServerApplication>::default()
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{
        body::Body,
        extract::Request,
        http::{header, StatusCode},
        response::Response,
    };
    use next_web_core::{
        filter::application_filter_chain::ApplicationFilterChain,
        http::buffer_request_body,
        traits::{
            filter::{HttpFilter, HttpFilterChain},
            http::{http_request::HttpRequest, http_response::HttpResponse},
        },
    };
    use next_web_security::{
        authentication::UsernamePasswordAuthenticationToken,
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder, web::builders::HttpSecurity,
        },
        core::{context::SecurityContextHolder, Authentication},
        web::security_filter_chain::SecurityFilterChain,
    };

    use super::{
        configure_oauth2_authorization_server, DEMO_CLIENT_ID, DEMO_CLIENT_SECRET,
        DEMO_REDIRECT_URI,
    };

    /// Builds the filters of the example the way a real application gets them.
    ///
    /// The security context holder filter is not part of the chain, so a test
    /// can set the resource owner of a request directly.
    ///
    /// # Returns
    ///
    /// The filters of the chain, in the order they run.
    fn security_filters() -> Arc<[Arc<dyn HttpFilter>]> {
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .request_cache(|_| {})
            .anonymous(|_| {});
        configure_oauth2_authorization_server(&mut http);

        Arc::from(http.build().get_filters().to_vec())
    }

    /// Runs one request through the filters of the chain.
    ///
    /// # Arguments
    ///
    /// * `filters` - The filters to run.
    /// * `request` - The request to run them against.
    ///
    /// # Returns
    ///
    /// The response the filters produced.
    async fn run(filters: &Arc<[Arc<dyn HttpFilter>]>, request: Request) -> Response {
        let chain = ApplicationFilterChain::new(Arc::clone(filters));
        let mut request = request;
        request.ready();
        let mut response = Response::new(Body::empty());

        chain
            .do_filter(&mut request, &mut response)
            .await
            .expect("the filter chain handles the request");

        response
    }

    /// Builds a request for a path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the request.
    fn request(path: &str) -> Request {
        Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("the request is built")
    }

    /// Builds a POST request with a url-encoded form body.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the request.
    /// * `form` - The parameters of the form.
    /// * `client` - The client id and secret of the client, if it authenticates.
    fn token_request(path: &str, form: &[(&str, &str)], client: Option<(&str, &str)>) -> Request {
        let mut builder = Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/x-www-form-urlencoded");
        if let Some((client_id, client_secret)) = client {
            use base64::{engine::general_purpose::STANDARD, Engine};

            let credentials = STANDARD.encode(format!("{client_id}:{client_secret}"));
            builder = builder.header("Authorization", format!("Basic {credentials}"));
        }

        let mut request = builder.body(Body::empty()).expect("the request is built");
        buffer_request_body(
            &mut request,
            form.iter()
                .map(|(name, value)| format!("{}={}", name, urlencoding::encode(value)))
                .collect::<Vec<_>>()
                .join("&"),
        );
        request
    }

    /// Returns the parameters of a redirect location.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    fn redirect_parameters(response: &Response) -> HashMap<String, String> {
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|location| location.to_str().ok())
            .expect("the response redirects");
        let query = location
            .split_once('?')
            .map(|(_, query)| query)
            .unwrap_or("");

        form_urlencoded::parse(query.as_bytes())
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect()
    }

    /// Returns the JSON body of a response.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    async fn json_body(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("the body of the response is read");
        serde_json::from_slice(&bytes).expect("the body of the response is JSON")
    }

    /// Signs the resource owner of the requests of a test in.
    ///
    /// The security context of the thread is set, and it is cleared when the
    /// guard is dropped.
    struct SignedInResourceOwner;

    impl SignedInResourceOwner {
        /// Signs the resource owner in.
        fn new() -> Self {
            let strategy = SecurityContextHolder::get_context_holder_strategy();
            let context = strategy.create_empty_context();
            let authentication: Arc<dyn Authentication> =
                Arc::new(UsernamePasswordAuthenticationToken::authenticated(
                    Arc::new("user".to_string()),
                    None,
                    Vec::new(),
                ));
            context.set_authentication(Some(authentication));
            strategy.set_context(context);

            Self
        }
    }

    impl Drop for SignedInResourceOwner {
        fn drop(&mut self) {
            SecurityContextHolder::get_context_holder_strategy().clear_context();
        }
    }

    /// Returns the authorization code an authorization request grants.
    ///
    /// # Arguments
    ///
    /// * `filters` - The filters to run.
    /// * `client_id` - The client of the authorization request.
    /// * `extra` - The extra parameters of the authorization request.
    async fn grant_code(
        filters: &Arc<[Arc<dyn HttpFilter>]>,
        client_id: &str,
        extra: &[(&str, &str)],
    ) -> String {
        let _resource_owner = SignedInResourceOwner::new();
        let mut parameters = vec![
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", DEMO_REDIRECT_URI),
            ("scope", "read"),
            ("state", "xyz"),
        ];
        parameters.extend_from_slice(extra);
        let query = parameters
            .iter()
            .map(|(name, value)| format!("{}={}", name, urlencoding::encode(value)))
            .collect::<Vec<_>>()
            .join("&");

        let response = run(filters, request(&format!("/oauth2/authorize?{query}"))).await;
        assert_eq!(response.status(), StatusCode::FOUND);
        let parameters = redirect_parameters(&response);
        assert_eq!(parameters.get("state").map(String::as_str), Some("xyz"));

        parameters
            .get("code")
            .cloned()
            .expect("the redirect carries the authorization code")
    }

    #[tokio::test]
    async fn the_metadata_endpoint_advertises_the_endpoints_of_the_server() {
        let filters = security_filters();

        let response = run(&filters, request("/.well-known/oauth-authorization-server")).await;

        assert_eq!(response.status(), StatusCode::OK);
        let metadata = json_body(response).await;
        assert_eq!(metadata["issuer"], super::DEMO_ISSUER);
        assert_eq!(
            metadata["token_endpoint"],
            format!("{}/oauth2/token", super::DEMO_ISSUER)
        );
        assert_eq!(metadata["grant_types_supported"][0], "authorization_code");
    }

    #[tokio::test]
    async fn the_jwk_set_endpoint_publishes_the_key_of_the_server() {
        let filters = security_filters();

        let response = run(&filters, request("/oauth2/jwks")).await;

        assert_eq!(response.status(), StatusCode::OK);
        let jwk_set = json_body(response).await;
        assert_eq!(jwk_set["keys"][0]["kty"], "oct");
        assert_eq!(jwk_set["keys"][0]["alg"], "HS256");
    }

    #[tokio::test]
    async fn the_authorization_endpoint_requires_a_resource_owner() {
        let filters = security_filters();

        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?response_type=code&client_id={DEMO_CLIENT_ID}&redirect_uri={DEMO_REDIRECT_URI}&scope=read"
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(header::WWW_AUTHENTICATE).is_some());
    }

    #[tokio::test]
    async fn the_authorization_endpoint_grants_a_code_to_a_resource_owner() {
        let filters = security_filters();

        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;

        assert!(!code.is_empty());
    }

    #[tokio::test]
    async fn the_authorization_endpoint_reports_an_unknown_client() {
        let filters = security_filters();
        let _resource_owner = SignedInResourceOwner::new();

        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?response_type=code&client_id=other&redirect_uri={DEMO_REDIRECT_URI}"
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = json_body(response).await;
        assert_eq!(error["error"], "invalid_request");
    }

    #[tokio::test]
    async fn the_authorization_endpoint_reports_an_unsupported_response_type() {
        let filters = security_filters();
        let _resource_owner = SignedInResourceOwner::new();

        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?response_type=token&client_id={DEMO_CLIENT_ID}&redirect_uri={DEMO_REDIRECT_URI}&state=xyz"
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        let parameters = redirect_parameters(&response);
        assert_eq!(
            parameters.get("error").map(String::as_str),
            Some("unsupported_response_type")
        );
        assert_eq!(parameters.get("state").map(String::as_str), Some("xyz"));
    }

    #[tokio::test]
    async fn the_token_endpoint_exchanges_the_code_for_a_token() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let token = json_body(response).await;
        assert_eq!(token["token_type"], "Bearer");
        assert_eq!(token["scope"], "read");
        assert!(token["access_token"]
            .as_str()
            .is_some_and(|value| value.starts_with("ey")));
        assert!(
            token["refresh_token"].as_str().is_some(),
            "a refresh token is issued"
        );
        assert!(token["expires_in"].as_u64().is_some());
    }

    #[tokio::test]
    async fn the_token_endpoint_does_not_exchange_the_code_twice() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", DEMO_REDIRECT_URI),
        ];

        let first = run(
            &filters,
            token_request(
                "/oauth2/token",
                &form,
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let second = run(
            &filters,
            token_request(
                "/oauth2/token",
                &form,
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(second.status(), StatusCode::BAD_REQUEST);
        let error = json_body(second).await;
        assert_eq!(error["error"], "invalid_grant");
    }

    #[tokio::test]
    async fn the_token_endpoint_requires_the_credentials_of_the_client() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                None,
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let error = json_body(response).await;
        assert_eq!(error["error"], "invalid_client");
    }

    #[tokio::test]
    async fn the_token_endpoint_rejects_a_wrong_client_secret() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                Some((DEMO_CLIENT_ID, "wrong")),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn the_token_endpoint_renews_the_token_of_a_refresh_token() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;
        let first = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let refresh_token = json_body(first).await["refresh_token"]
            .as_str()
            .expect("the authorization code grant issues a refresh token")
            .to_string();

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", refresh_token.as_str()),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let token = json_body(response).await;
        assert!(token["access_token"].as_str().is_some());
    }

    #[tokio::test]
    async fn the_token_endpoint_issues_a_token_to_the_client_itself() {
        let filters = security_filters();

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[("grant_type", "client_credentials"), ("scope", "read")],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let token = json_body(response).await;
        assert_eq!(token["scope"], "read");
        assert!(token["access_token"].as_str().is_some());
        assert!(token.get("refresh_token").is_none());
    }

    #[tokio::test]
    async fn the_token_endpoint_reports_an_unsupported_grant() {
        let filters = security_filters();

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "password"),
                    ("username", "user"),
                    ("password", "password"),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = json_body(response).await;
        assert_eq!(error["error"], "unsupported_grant_type");
    }

    #[tokio::test]
    async fn the_token_endpoint_exchanges_a_code_of_a_public_client_with_a_proof_key() {
        use sha2::{Digest, Sha256};

        let filters = security_filters();
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = next_web_security::oauth2::server::authorization::jose::base64_url(
            &Sha256::digest(verifier.as_bytes()),
        );
        let code = grant_code(
            &filters,
            "public-client",
            &[
                ("code_challenge", &challenge),
                ("code_challenge_method", "S256"),
            ],
        )
        .await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                    ("client_id", "public-client"),
                    ("code_verifier", verifier),
                ],
                None,
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let token = json_body(response).await;
        assert!(token["access_token"].as_str().is_some());
    }

    #[tokio::test]
    async fn the_token_endpoint_rejects_a_code_of_a_public_client_without_the_proof_key() {
        let filters = security_filters();
        let code = grant_code(
            &filters,
            "public-client",
            &[
                ("code_challenge", "challenge"),
                ("code_challenge_method", "plain"),
            ],
        )
        .await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                    ("client_id", "public-client"),
                    ("code_verifier", "other"),
                ],
                None,
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = json_body(response).await;
        assert_eq!(error["error"], "invalid_grant");
    }

    #[tokio::test]
    async fn the_introspection_endpoint_reports_the_state_of_a_token() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;
        let token_response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let access_token = json_body(token_response).await["access_token"]
            .as_str()
            .expect("the grant issues an access token")
            .to_string();

        let response = run(
            &filters,
            token_request(
                "/oauth2/introspect",
                &[("token", access_token.as_str())],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let introspection = json_body(response).await;
        assert_eq!(introspection["active"], true);
        assert_eq!(introspection["client_id"], DEMO_CLIENT_ID);
        assert_eq!(introspection["scope"], "read");
        assert_eq!(introspection["sub"], "user");
    }

    #[tokio::test]
    async fn the_revocation_endpoint_revokes_a_token() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;
        let token_response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let access_token = json_body(token_response).await["access_token"]
            .as_str()
            .expect("the grant issues an access token")
            .to_string();

        let response = run(
            &filters,
            token_request(
                "/oauth2/revoke",
                &[("token", access_token.as_str())],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = run(
            &filters,
            token_request(
                "/oauth2/introspect",
                &[("token", access_token.as_str())],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let introspection = json_body(response).await;
        assert_eq!(introspection["active"], false);
    }

    #[tokio::test]
    async fn the_resource_server_accepts_the_token_the_server_issued() {
        let filters = security_filters();
        let code = grant_code(&filters, DEMO_CLIENT_ID, &[]).await;
        let token_response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", &code),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let access_token = json_body(token_response).await["access_token"]
            .as_str()
            .expect("the grant issues an access token")
            .to_string();

        let response = run(
            &filters,
            Request::builder()
                .uri("/api/me")
                .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .expect("the request is built"),
        )
        .await;

        assert!(!HttpResponse::is_committed(&response));
    }
}

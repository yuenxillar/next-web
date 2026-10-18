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
    Application, NextWebApplication,
    core::{ApplicationContext, ApplyRouter, Ordered},
    macros::{autoconfigure::configuration, bind::singleton},
};
use next_web_context::{ApplicationEvent, ApplicationEventPublisher};
use next_web_core::error::BoxError;
use next_web_security::{
    authentication::dao::DaoAuthenticationProvider,
    config::web::{HttpSecurityBuilder, WebSecurityConfigurer, builders::HttpSecurity},
    core::userdetails::User,
    crypto::bcrypt::BCryptPasswordEncoder,
    crypto::password::PasswordEncoder,
    oauth2::{
        client::{
            ClientRegistration, InMemoryClientRegistrationRepository,
            registration::ClientAuthenticationMethod as ClientAuthMethod,
        },
        server::{
            authorization::{
                AuthorizationGrantType, AuthorizationServerSettings, ClientAuthenticationMethod,
                ClientSettings, InMemoryRegisteredClientRepository as InMemoryRegisteredClients,
                JwtSigner, RegisteredClient, RsaJwtSigner, SignerJwkSource,
            },
            resource::jwt::NimbusJwtDecoder,
        },
    },
    provisioning::InMemoryUserDetailsManager,
};

/// The identifier of the demo client.
pub const DEMO_CLIENT_ID: &str = "demo-client";

/// The secret of the demo client.
pub const DEMO_CLIENT_SECRET: &str = "secret";

/// The redirect URI of the demo client.
pub const DEMO_REDIRECT_URI: &str = "http://127.0.0.1:11000/app/callback";

/// The client that has to ask the resource owner for consent.
pub const CONSENT_CLIENT_ID: &str = "consent-client";

/// The secret of the client that has to ask for consent.
pub const CONSENT_CLIENT_SECRET: &str = "consent-secret";

/// The client the application registers as its own login.
pub const LOGIN_CLIENT_ID: &str = "as-demo";

/// The secret of the client the application registers as its own login.
pub const LOGIN_CLIENT_SECRET: &str = "as-secret";

/// The redirect URI of the login of the application, which is the one the
/// authorization server knows the client by.
pub const DEMO_LOGIN_REDIRECT_URI: &str = "http://127.0.0.1:11000/app/login/oauth2/code/as-demo";

/// The client that proves its identity with a signed assertion.
pub const ASSERTION_CLIENT_ID: &str = "assertion-client";

/// The secret the assertion client signs its assertions with.
pub const ASSERTION_CLIENT_SECRET: &str = "assertion-secret";

/// The initial access token a client registers itself with.
pub const DEMO_INITIAL_ACCESS_TOKEN: &str = "initial-access-token";

/// The issuer of the authorization server.
pub const DEMO_ISSUER: &str = "http://127.0.0.1:11000/app";

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
            .route("/consent", get(consent_page))
    }
}

/// Serves the page the resource owner approves the scopes of a client on.
///
/// The page is what an application serves itself. It asks the authorization
/// endpoint of the server to grant the scopes it was asked for: the form that
/// is submitted with the approved scopes asks for the authorization code, and
/// the form that is submitted without any scope denies the request.
async fn consent_page(
    axum::extract::Query(query): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Html<String> {
    let client_id = query.get("client_id").cloned().unwrap_or_default();
    let state = query.get("state").cloned().unwrap_or_default();
    let scopes = query
        .get("scope")
        .map(|scope| {
            scope
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let scope_inputs = scopes
        .iter()
        .map(|scope| {
            format!(
                "<label><input type=\"checkbox\" name=\"scope\" value=\"{scope}\" checked> \
                 {scope}</label><br>"
            )
        })
        .collect::<String>();

    axum::response::Html(format!(
        "<!doctype html><html><body><h1>Authorize {client_id}</h1>\
         <form method=\"post\" action=\"/app/oauth2/authorize\">\
         <input type=\"hidden\" name=\"client_id\" value=\"{client_id}\">\
         <input type=\"hidden\" name=\"state\" value=\"{state}\">\
         {scope_inputs}\
         <button type=\"submit\">Approve</button></form>\
         <form method=\"post\" action=\"/app/oauth2/authorize\">\
         <input type=\"hidden\" name=\"client_id\" value=\"{client_id}\">\
         <input type=\"hidden\" name=\"state\" value=\"{state}\">\
         <button type=\"submit\">Deny</button></form>\
         </body></html>"
    ))
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
        .authorization_grant_type(AuthorizationGrantType::DeviceCode)
        .redirect_uri(DEMO_REDIRECT_URI)
        .scope("read")
        .set_client_settings(
            ClientSettings::builder()
                .require_proof_key(false)
                .require_authorization_consent(false)
                .build(),
        );

    let public_client = RegisteredClient::with_client_id("public-client")
        .set_client_name("Public Client")
        .client_authentication_method(ClientAuthenticationMethod::None)
        .authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
        .redirect_uri(DEMO_REDIRECT_URI)
        .scope("read")
        .set_client_settings(
            ClientSettings::builder()
                .require_proof_key(true)
                .require_authorization_consent(false)
                .build(),
        );

    let consent_client = RegisteredClient::with_client_id(CONSENT_CLIENT_ID)
        .set_client_secret(format!("{{noop}}{CONSENT_CLIENT_SECRET}"))
        .set_client_name("Consent Client")
        .client_authentication_method(ClientAuthenticationMethod::ClientSecretBasic)
        .authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
        .redirect_uri(DEMO_REDIRECT_URI)
        .scope("read")
        .set_client_settings(
            ClientSettings::builder()
                .require_proof_key(false)
                .require_authorization_consent(true)
                .build(),
        );

    // The client the application itself logs in with, which is a client of the
    // authorization server it serves.
    let login_client = RegisteredClient::with_client_id(LOGIN_CLIENT_ID)
        .set_client_secret(format!("{{noop}}{LOGIN_CLIENT_SECRET}"))
        .set_client_name("Application Login")
        .client_authentication_method(ClientAuthenticationMethod::ClientSecretBasic)
        .authorization_grant_type(AuthorizationGrantType::AuthorizationCode)
        .authorization_grant_type(AuthorizationGrantType::RefreshToken)
        .redirect_uri(DEMO_LOGIN_REDIRECT_URI)
        .scope("openid")
        .scope("read")
        .set_client_settings(
            ClientSettings::builder()
                .require_proof_key(false)
                .require_authorization_consent(false)
                .build(),
        );

    // The client that proves its identity with a signed assertion. Its key is
    // the secret the assertion is signed with, so the example can sign an
    // assertion without a key pair.
    let assertion_client = RegisteredClient::with_client_id(ASSERTION_CLIENT_ID)
        .set_client_name("Assertion Client")
        .client_authentication_method(ClientAuthenticationMethod::PrivateKeyJwt)
        .authorization_grant_type(AuthorizationGrantType::ClientCredentials)
        .scope("read")
        .set_client_settings(
            ClientSettings::builder()
                .require_proof_key(false)
                .require_authorization_consent(false)
                .jwk_set(crate::demo_assertion_jwk_set())
                .build(),
        );

    vec![
        demo_client,
        public_client,
        consent_client,
        login_client,
        assertion_client,
    ]
}

/// Returns the keys the assertion client signs its assertions with.
pub fn demo_assertion_jwk_set() -> next_web_security::oauth2::jose::JwkSet {
    next_web_security::oauth2::jose::JwkSet::new(vec![
        next_web_security::oauth2::jose::Jwk::symmetric(ASSERTION_CLIENT_SECRET),
    ])
}

/// Returns the settings of the authorization server of the example.
pub fn demo_authorization_server_settings() -> AuthorizationServerSettings {
    AuthorizationServerSettings::builder()
        .issuer(DEMO_ISSUER)
        .scopes(["openid", "read"])
        // The client registration endpoint is served only for a request that
        // presents this token, so the endpoint is not open.
        .client_registration_initial_access_token(DEMO_INITIAL_ACCESS_TOKEN)
        .build()
}

/// Returns the signer the tokens of the example are signed with.
///
/// The key pair is generated once while the application starts, and only its
/// public part is published by the JWK set endpoint. A deployment reads the key
/// pair from its configuration instead, so that its tokens stay valid across a
/// restart.
pub fn demo_jwt_signer() -> Arc<dyn JwtSigner> {
    static SIGNER: std::sync::OnceLock<Arc<dyn JwtSigner>> = std::sync::OnceLock::new();

    SIGNER
        .get_or_init(|| {
            Arc::new(
                RsaJwtSigner::generate(2048)
                    .expect("the demo key pair of the authorization server is generated"),
            )
        })
        .clone()
}

/// Returns the registration the application logs in with.
///
/// The application is a client of the authorization server it serves: the
/// registration points at the endpoints of the server, and the login of the
/// application exchanges the authorization code it receives for a token at the
/// token endpoint of the server.
pub fn demo_login_client() -> ClientRegistration {
    ClientRegistration::new(
        LOGIN_CLIENT_ID,
        "Application Login",
        format!("{DEMO_ISSUER}/oauth2/authorize"),
    )
    .set_client_id(LOGIN_CLIENT_ID)
    .set_client_secret(LOGIN_CLIENT_SECRET)
    .set_token_uri(format!("{DEMO_ISSUER}/oauth2/token"))
    .set_redirect_uri(DEMO_LOGIN_REDIRECT_URI)
    .set_scopes(["openid", "read"])
    .set_user_info_uri(format!("{DEMO_ISSUER}/userinfo"))
    // The identifier token of the grant is verified with the keys of the
    // server, and the issuer of the token is the issuer of the server.
    .set_issuer_uri(DEMO_ISSUER)
    .set_jwk_set_uri(format!("{DEMO_ISSUER}/oauth2/jwks"))
    .set_client_authentication_method(ClientAuthMethod::ClientSecretBasic)
}

/// Returns the resource owner the example logs in.
///
/// # Returns
///
/// The user details service of the example, whose user is `user` with the
/// password `password`.
pub fn demo_user_details_service()
-> Arc<dyn next_web_security::core::userdetails::UserDetailsService> {
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
    configure_oauth2_authorization_server_with_settings(http, demo_authorization_server_settings());
}

/// Applies the configuration of the example with the given settings.
///
/// The tests that cover a setting which changes the endpoints of the server
/// build their chain with the settings they need, the way the example does.
///
/// # Arguments
///
/// * `http` - The HTTP security the configuration is applied to.
/// * `settings` - The settings of the authorization server.
pub fn configure_oauth2_authorization_server_with_settings(
    http: &mut HttpSecurity,
    settings: AuthorizationServerSettings,
) {
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
                // The login of the application exchanges its authorization
                // code here, before the resource owner is authenticated.
                "/login/oauth2/code/**",
                AuthorizationServerSettings::DEFAULT_AUTHORIZATION_SERVER_METADATA_ENDPOINT,
                AuthorizationServerSettings::DEFAULT_OPENID_CONFIGURATION_ENDPOINT,
                AuthorizationServerSettings::DEFAULT_JWK_SET_ENDPOINT,
            ])
            .permit_all()
            .any_request()
            .authenticated();
    })
    .http_basic(|_| {})
    .oauth2_authorization_server(move |authorization_server| {
        authorization_server
            .registered_client_repository(Arc::new(InMemoryRegisteredClients::new(
                demo_registered_clients(),
            )))
            .authorization_server_settings(settings.clone())
            .jwt_signer(demo_jwt_signer())
            // The consent page of the example is served by the application, and
            // a client that requires consent is asked on it.
            .consent_page("/consent");
    })
    .oauth2_login(|login| {
        // The application is a client of the authorization server it serves, so
        // its login exchanges the code it receives at the token endpoint of the
        // server.
        login.client_registration_repository(Arc::new(InMemoryClientRegistrationRepository::new(
            [demo_login_client()],
        )));
    })
    .oauth2_resource_server(|resource_server| {
        resource_server.jwt(|jwt| {
            // The resource server of the same application verifies the tokens
            // with the public keys the authorization server publishes.
            jwt.decoder(Arc::new(
                NimbusJwtDecoder::default()
                    .with_jwk_source(Arc::new(SignerJwkSource::new(demo_jwt_signer()))),
            ));
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
        http::{StatusCode, header},
        response::Response,
    };
    use next_web_core::{
        filter::application_filter_chain::ApplicationFilterChain,
        http::buffer_request_body,
        traits::{
            filter::{HttpFilter, HttpFilterChain},
            http::{HttpRequest, HttpResponse},
        },
    };
    use next_web_security::{
        authentication::UsernamePasswordAuthenticationToken,
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder, web::builders::HttpSecurity,
        },
        core::{Authentication, Principal, context::SecurityContextHolder},
        oauth2::server::resource::{
            OpaqueTokenIntrospector,
            jwt::{JwtDecoder, NimbusJwtDecoder},
        },
        web::security_filter_chain::SecurityFilterChain,
    };

    use super::{
        ASSERTION_CLIENT_ID, ASSERTION_CLIENT_SECRET, CONSENT_CLIENT_ID, CONSENT_CLIENT_SECRET,
        DEMO_CLIENT_ID, DEMO_CLIENT_SECRET, DEMO_INITIAL_ACCESS_TOKEN, DEMO_ISSUER,
        DEMO_LOGIN_REDIRECT_URI, DEMO_REDIRECT_URI, LOGIN_CLIENT_ID, LOGIN_CLIENT_SECRET,
        configure_oauth2_authorization_server, configure_oauth2_authorization_server_with_settings,
    };

    /// Reads the claims of an identifier token.
    ///
    /// # Arguments
    ///
    /// * `id_token` - The identifier token of a grant.
    fn claims_of(id_token: &str) -> serde_json::Value {
        let payload = next_web_security::oauth2::jose::base64_url_decode(
            id_token.split('.').nth(1).expect("the token has a payload"),
        )
        .expect("the payload of the token is base64url");

        serde_json::from_slice(&payload).expect("the payload of the token is JSON")
    }

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
            use base64::{Engine, engine::general_purpose::STANDARD};

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
        grant_code_with_scopes(filters, client_id, "read", extra).await
    }

    /// Returns the authorization code an authorization request grants for the
    /// given scopes.
    ///
    /// # Arguments
    ///
    /// * `filters` - The filters to run.
    /// * `client_id` - The client of the authorization request.
    /// * `scopes` - The scopes of the authorization request.
    /// * `extra` - The extra parameters of the authorization request.
    async fn grant_code_with_scopes(
        filters: &Arc<[Arc<dyn HttpFilter>]>,
        client_id: &str,
        scopes: &str,
        extra: &[(&str, &str)],
    ) -> String {
        grant_code_for_redirect_uri(filters, client_id, scopes, DEMO_REDIRECT_URI, extra).await
    }

    /// Returns the authorization code an authorization request grants for the
    /// given scopes and redirect URI.
    ///
    /// # Arguments
    ///
    /// * `filters` - The filters to run.
    /// * `client_id` - The client of the authorization request.
    /// * `scopes` - The scopes of the authorization request.
    /// * `redirect_uri` - The redirect URI of the client.
    /// * `extra` - The extra parameters of the authorization request.
    async fn grant_code_for_redirect_uri(
        filters: &Arc<[Arc<dyn HttpFilter>]>,
        client_id: &str,
        scopes: &str,
        redirect_uri: &str,
        extra: &[(&str, &str)],
    ) -> String {
        let _resource_owner = SignedInResourceOwner::new();
        let mut parameters = vec![
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("scope", scopes),
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

    /// Returns the state of a client that is asked for consent.
    ///
    /// # Arguments
    ///
    /// * `filters` - The filters to run.
    /// * `client_id` - The client of the authorization request.
    async fn consent_state(filters: &Arc<[Arc<dyn HttpFilter>]>, client_id: &str) -> String {
        let response = run(
            filters,
            request(&format!(
                "/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri={DEMO_REDIRECT_URI}&scope=read&state=xyz"
            )),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FOUND);
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|location| location.to_str().ok())
            .expect("the response redirects");
        assert!(
            location.starts_with("/consent?"),
            "unexpected page: {location}"
        );

        form_urlencoded::parse(
            location
                .split_once('?')
                .expect("the page has a query")
                .1
                .as_bytes(),
        )
        .find(|(name, _)| name == "state")
        .map(|(_, value)| value.into_owned())
        .expect("the consent page carries the state")
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
        assert_eq!(jwk_set["keys"][0]["kty"], "RSA");
        assert_eq!(jwk_set["keys"][0]["alg"], "RS256");
        assert!(
            jwk_set["keys"][0]["n"].is_string(),
            "the modulus is published"
        );
        assert!(
            jwk_set["keys"][0].get("k").is_none(),
            "the private part is not published"
        );
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
    async fn a_client_that_requires_consent_is_asked_on_the_consent_page() {
        let filters = security_filters();
        let _resource_owner = SignedInResourceOwner::new();

        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?response_type=code&client_id={CONSENT_CLIENT_ID}&redirect_uri={DEMO_REDIRECT_URI}&scope=read&state=xyz"
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|location| location.to_str().ok())
            .expect("the response redirects");
        assert!(
            location.starts_with("/consent?"),
            "unexpected page: {location}"
        );
        let parameters: HashMap<String, String> =
            form_urlencoded::parse(location.split_once('?').unwrap().1.as_bytes())
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect();
        assert_eq!(
            parameters.get("client_id").map(String::as_str),
            Some(CONSENT_CLIENT_ID)
        );
        assert_eq!(parameters.get("scope").map(String::as_str), Some("read"));
        assert_eq!(parameters.get("state").map(String::as_str), Some("xyz"));
    }

    #[tokio::test]
    async fn the_resource_owner_grants_the_scopes_of_a_client_on_the_consent_page() {
        let filters = security_filters();
        let _resource_owner = SignedInResourceOwner::new();
        let state = consent_state(&filters, CONSENT_CLIENT_ID).await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/authorize",
                &[
                    ("client_id", CONSENT_CLIENT_ID),
                    ("state", state.as_str()),
                    ("scope", "read"),
                ],
                None,
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        let parameters = redirect_parameters(&response);
        assert!(parameters.get("code").is_some(), "the code is granted");
        assert_eq!(
            parameters.get("state").map(String::as_str),
            Some(state.as_str())
        );

        // The code of the consent is exchanged for a token like any other code.
        let code = parameters.get("code").expect("the code is granted").clone();
        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", code.as_str()),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                ],
                Some((CONSENT_CLIENT_ID, CONSENT_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let token = json_body(response).await;
        assert_eq!(token["scope"], "read");
    }

    #[tokio::test]
    async fn the_consent_of_a_resource_owner_is_remembered() {
        let filters = security_filters();
        let state = {
            let _resource_owner = SignedInResourceOwner::new();
            consent_state(&filters, CONSENT_CLIENT_ID).await
        };
        let approved = {
            let _resource_owner = SignedInResourceOwner::new();
            run(
                &filters,
                token_request(
                    "/oauth2/authorize",
                    &[
                        ("client_id", CONSENT_CLIENT_ID),
                        ("state", state.as_str()),
                        ("scope", "read"),
                    ],
                    None,
                ),
            )
            .await
        };
        assert_eq!(approved.status(), StatusCode::FOUND);

        // The scopes were approved, so the next authorization request is
        // granted without asking the resource owner again.
        let _resource_owner = SignedInResourceOwner::new();
        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?response_type=code&client_id={CONSENT_CLIENT_ID}&redirect_uri={DEMO_REDIRECT_URI}&scope=read&state=next"
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        let parameters = redirect_parameters(&response);
        assert!(parameters.get("code").is_some(), "the code is granted");
    }

    #[tokio::test]
    async fn the_resource_owner_denies_the_scopes_of_a_client_on_the_consent_page() {
        let filters = security_filters();
        let _resource_owner = SignedInResourceOwner::new();
        let state = consent_state(&filters, CONSENT_CLIENT_ID).await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/authorize",
                &[("client_id", CONSENT_CLIENT_ID), ("state", state.as_str())],
                None,
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        let parameters = redirect_parameters(&response);
        assert_eq!(
            parameters.get("error").map(String::as_str),
            Some("access_denied")
        );
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
        assert!(
            token["access_token"]
                .as_str()
                .is_some_and(|value| value.starts_with("ey"))
        );
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
        let challenge =
            next_web_security::oauth2::jose::base64_url(&Sha256::digest(verifier.as_bytes()));
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

    #[tokio::test]
    async fn the_discovery_document_advertises_the_openid_connect_endpoints() {
        let filters = security_filters();

        let response = run(&filters, request("/.well-known/openid-configuration")).await;

        assert_eq!(response.status(), StatusCode::OK);
        let document = json_body(response).await;
        assert_eq!(document["issuer"], super::DEMO_ISSUER);
        assert_eq!(
            document["userinfo_endpoint"],
            format!("{}/userinfo", super::DEMO_ISSUER)
        );
        assert_eq!(
            document["end_session_endpoint"],
            format!("{}/connect/logout", super::DEMO_ISSUER)
        );
        assert_eq!(document["scopes_supported"][0], "openid");
        assert_eq!(document["response_types_supported"][0], "code");
    }

    #[tokio::test]
    async fn the_token_response_of_an_openid_grant_carries_an_identifier_token() {
        let filters = security_filters();
        let code = grant_code_for_redirect_uri(
            &filters,
            LOGIN_CLIENT_ID,
            "openid read",
            DEMO_LOGIN_REDIRECT_URI,
            &[],
        )
        .await;

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", code.as_str()),
                    ("redirect_uri", DEMO_LOGIN_REDIRECT_URI),
                ],
                Some((LOGIN_CLIENT_ID, LOGIN_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let token = json_body(response).await;
        let id_token = token["id_token"]
            .as_str()
            .expect("the grant of an OpenID Connect client issues an identifier token");
        let claims = claims_of(id_token);
        assert_eq!(claims["sub"], "user");
        assert_eq!(claims["aud"], LOGIN_CLIENT_ID);
        assert_eq!(claims["iss"], super::DEMO_ISSUER);
    }

    #[tokio::test]
    async fn the_identifier_token_of_the_server_is_verified_with_the_keys_of_the_server() {
        use next_web_security::oauth2::{DefaultOidcIdTokenValidator, OidcIdTokenValidator};

        let filters = security_filters();
        let base = serve(filters.clone()).await;

        // The grant of the login client of the example yields an identifier
        // token, the way the login of the application receives one.
        let code = grant_code_for_redirect_uri(
            &filters,
            LOGIN_CLIENT_ID,
            "openid read",
            DEMO_LOGIN_REDIRECT_URI,
            &[],
        )
        .await;
        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", code.as_str()),
                    ("redirect_uri", DEMO_LOGIN_REDIRECT_URI),
                ],
                Some((LOGIN_CLIENT_ID, LOGIN_CLIENT_SECRET)),
            ),
        )
        .await;
        let token = json_body(response).await;
        let id_token = token["id_token"]
            .as_str()
            .expect("the grant of an OpenID Connect client issues an identifier token")
            .to_string();
        let access_token = token["access_token"]
            .as_str()
            .expect("the grant issues an access token")
            .to_string();

        // The registration of the login client names the keys of the server of
        // the listener, so the validator reads them over HTTP.
        let registration = super::demo_login_client()
            .set_jwk_set_uri(format!("{base}/oauth2/jwks"))
            .set_issuer_uri(super::DEMO_ISSUER);
        let validator = DefaultOidcIdTokenValidator::default();
        let validated = validator
            .validate(&registration, &id_token, None, Some(access_token.as_str()))
            .await
            .expect("the identifier token of the server is verified");

        assert_eq!(validated.issuer(), super::DEMO_ISSUER);
        assert_eq!(validated.subject(), "user");
        assert_eq!(validated.audience(), [LOGIN_CLIENT_ID]);

        // A token whose claims were changed after it was signed is refused.
        let mut parts = id_token.split('.').collect::<Vec<_>>();
        let payload = next_web_security::oauth2::jose::base64_url_decode(parts[1])
            .expect("the payload of the token is base64url");
        let mut claims: serde_json::Value =
            serde_json::from_slice(&payload).expect("the payload of the token is JSON");
        claims["sub"] = serde_json::json!("admin");
        let payload = next_web_security::oauth2::jose::base64_url(claims.to_string().as_bytes());
        parts[1] = &payload;
        let tampered = parts.join(".");

        assert!(
            validator
                .validate(&registration, &tampered, None, Some(access_token.as_str()))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn the_user_info_endpoint_answers_for_an_access_token() {
        let filters = security_filters();
        let code = grant_code_for_redirect_uri(
            &filters,
            LOGIN_CLIENT_ID,
            "openid read",
            DEMO_LOGIN_REDIRECT_URI,
            &[],
        )
        .await;
        let token_response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "authorization_code"),
                    ("code", code.as_str()),
                    ("redirect_uri", DEMO_LOGIN_REDIRECT_URI),
                ],
                Some((LOGIN_CLIENT_ID, LOGIN_CLIENT_SECRET)),
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
                .uri("/userinfo")
                .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .expect("the request is built"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let user_info = json_body(response).await;
        assert_eq!(user_info["sub"], "user");
        assert_eq!(user_info["name"], "user");
    }

    #[tokio::test]
    async fn the_user_info_endpoint_rejects_an_unknown_access_token() {
        let filters = security_filters();

        let response = run(
            &filters,
            Request::builder()
                .uri("/userinfo")
                .header(header::AUTHORIZATION, "Bearer not-a-token")
                .body(Body::empty())
                .expect("the request is built"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn the_logout_endpoint_ends_the_session_of_the_resource_owner() {
        let filters = security_filters();
        let _resource_owner = SignedInResourceOwner::new();

        let response = run(
            &filters,
            request(&format!(
                "/connect/logout?post_logout_redirect_uri={DEMO_LOGIN_REDIRECT_URI}"
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(header::LOCATION)
                .and_then(|location| location.to_str().ok()),
            Some(DEMO_LOGIN_REDIRECT_URI)
        );
    }

    #[tokio::test]
    async fn the_logout_endpoint_does_not_send_the_resource_owner_elsewhere() {
        let filters = security_filters();
        let _resource_owner = SignedInResourceOwner::new();

        let response = run(
            &filters,
            request("/connect/logout?post_logout_redirect_uri=https://other.example/out"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Serves the filters of the example over HTTP.
    ///
    /// The tests that reach the endpoints of the server the way a client or a
    /// resource server does run the filters behind a real HTTP listener.
    ///
    /// # Arguments
    ///
    /// * `filters` - The filters of the chain.
    ///
    /// # Returns
    ///
    /// The base URI of the listener.
    async fn serve(filters: Arc<[Arc<dyn HttpFilter>]>) -> String {
        async fn handle(
            axum::extract::State(filters): axum::extract::State<Arc<[Arc<dyn HttpFilter>]>>,
            request: Request,
        ) -> Response {
            // The filters read the body of a form post and the body of a JSON
            // document, so the body of the request is buffered the way the web
            // layer of the framework buffers it.
            let (parts, body) = request.into_parts();
            let bytes = axum::body::to_bytes(body, 64 * 1024)
                .await
                .expect("the body of the request is read");
            let mut request = Request::from_parts(parts, Body::from(bytes.clone()));
            next_web_core::http::buffer_request_body(&mut request, bytes);

            let chain = ApplicationFilterChain::new(filters);
            next_web_core::http::HttpRequest::ready(&mut request);
            let mut response = Response::new(Body::empty());

            chain
                .do_filter(&mut request, &mut response)
                .await
                .expect("the filter chain handles the request");

            response
        }

        let app = axum::Router::new().fallback(handle).with_state(filters);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the server listens");
        let address = listener.local_addr().expect("the server has an address");

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        format!("http://{address}")
    }

    /// Returns the access token of the client credentials grant of the demo
    /// client, obtained over HTTP.
    ///
    /// # Arguments
    ///
    /// * `base` - The base URI of the server.
    async fn fetch_demo_token(base: &str) -> String {
        let token: serde_json::Value = reqwest::Client::new()
            .post(format!("{base}/oauth2/token"))
            .basic_auth(DEMO_CLIENT_ID, Some(DEMO_CLIENT_SECRET))
            .form(&[("grant_type", "client_credentials"), ("scope", "read")])
            .send()
            .await
            .expect("the token endpoint of the server is reached")
            .error_for_status()
            .expect("the token endpoint issues a token")
            .json()
            .await
            .expect("the token response is a JSON document");

        token["access_token"]
            .as_str()
            .expect("the token response carries the access token")
            .to_string()
    }

    #[tokio::test]
    async fn the_pushed_authorization_request_endpoint_returns_a_request_uri() {
        let filters = security_filters();

        let response = run(
            &filters,
            token_request(
                "/oauth2/par",
                &[
                    ("response_type", "code"),
                    ("client_id", DEMO_CLIENT_ID),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                    ("scope", "read"),
                    ("state", "xyz"),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = json_body(response).await;
        assert!(
            body["request_uri"]
                .as_str()
                .expect("the response carries the request URI")
                .starts_with("urn:ietf:params:oauth:request_uri:")
        );
        assert_eq!(body["expires_in"], 90);
    }

    #[tokio::test]
    async fn the_authorization_endpoint_serves_a_pushed_authorization_request() {
        let filters = security_filters();

        let response = run(
            &filters,
            token_request(
                "/oauth2/par",
                &[
                    ("response_type", "code"),
                    ("client_id", DEMO_CLIENT_ID),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                    ("scope", "read"),
                    ("state", "xyz"),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let request_uri = json_body(response).await["request_uri"]
            .as_str()
            .expect("the response carries the request URI")
            .to_string();

        let _resource_owner = SignedInResourceOwner::new();
        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?client_id={DEMO_CLIENT_ID}&request_uri={}",
                urlencoding::encode(&request_uri)
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        let parameters = redirect_parameters(&response);
        assert_eq!(parameters.get("state").map(String::as_str), Some("xyz"));
        let code = parameters
            .get("code")
            .cloned()
            .expect("the authorization code is granted");

        // The code of the pushed request is exchanged the way the code of an
        // ordinary authorization request is.
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
    }

    #[tokio::test]
    async fn a_pushed_authorization_request_is_used_only_once() {
        let filters = security_filters();

        let response = run(
            &filters,
            token_request(
                "/oauth2/par",
                &[
                    ("response_type", "code"),
                    ("client_id", DEMO_CLIENT_ID),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                    ("scope", "read"),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let request_uri = json_body(response).await["request_uri"]
            .as_str()
            .expect("the response carries the request URI")
            .to_string();
        let uri = format!(
            "/oauth2/authorize?client_id={DEMO_CLIENT_ID}&request_uri={}",
            urlencoding::encode(&request_uri)
        );

        {
            let _resource_owner = SignedInResourceOwner::new();
            let response = run(&filters, request(&uri)).await;
            assert_eq!(response.status(), StatusCode::FOUND);
        }

        let _resource_owner = SignedInResourceOwner::new();
        let response = run(&filters, request(&uri)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Builds the filters of the example with the given settings of the
    /// authorization server.
    ///
    /// # Arguments
    ///
    /// * `settings` - The settings of the authorization server.
    fn security_filters_with_settings(
        settings: next_web_security::oauth2::server::authorization::AuthorizationServerSettings,
    ) -> Arc<[Arc<dyn HttpFilter>]> {
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .request_cache(|_| {})
            .anonymous(|_| {});
        configure_oauth2_authorization_server_with_settings(&mut http, settings);

        Arc::from(http.build().get_filters().to_vec())
    }

    #[tokio::test]
    async fn the_authorization_endpoint_requires_a_pushed_request_when_configured() {
        use next_web_security::oauth2::server::authorization::AuthorizationServerSettings;

        let settings = AuthorizationServerSettings::builder()
            .issuer(DEMO_ISSUER)
            .scopes(["openid", "read"])
            .client_registration_initial_access_token(DEMO_INITIAL_ACCESS_TOKEN)
            .require_pushed_authorization_requests(true)
            .build();
        let filters = security_filters_with_settings(settings);

        // The metadata of the server announces that a request has to be pushed.
        let response = run(&filters, request("/.well-known/oauth-authorization-server")).await;
        assert_eq!(
            json_body(response).await["require_pushed_authorization_requests"],
            true
        );

        // An authorization request that was not pushed is refused.
        let _resource_owner = SignedInResourceOwner::new();
        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?response_type=code&client_id={DEMO_CLIENT_ID}&redirect_uri={DEMO_REDIRECT_URI}&scope=read&state=xyz"
            )),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["error"], "invalid_request");

        // The request is served once its request URI is presented.
        let response = run(
            &filters,
            token_request(
                "/oauth2/par",
                &[
                    ("response_type", "code"),
                    ("client_id", DEMO_CLIENT_ID),
                    ("redirect_uri", DEMO_REDIRECT_URI),
                    ("scope", "read"),
                    ("state", "xyz"),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let request_uri = json_body(response).await["request_uri"]
            .as_str()
            .expect("the response carries the request URI")
            .to_string();
        let response = run(
            &filters,
            request(&format!(
                "/oauth2/authorize?client_id={DEMO_CLIENT_ID}&request_uri={}",
                urlencoding::encode(&request_uri)
            )),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
    }

    #[tokio::test]
    async fn the_device_grant_issues_a_token_after_the_resource_owner_approved() {
        let filters = security_filters();

        let response = run(
            &filters,
            token_request(
                "/oauth2/device_authorization",
                &[("scope", "read")],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        let device_code = body["device_code"]
            .as_str()
            .expect("the response carries the device code")
            .to_string();
        let user_code = body["user_code"]
            .as_str()
            .expect("the response carries the user code")
            .to_string();
        assert_eq!(body["interval"], 5);
        assert!(
            body["verification_uri"]
                .as_str()
                .expect("the response carries the verification URI")
                .ends_with("/oauth2/device_verification")
        );

        // The device polls before the resource owner answered.
        let poll = |device_code: &str| {
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", device_code),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            )
        };
        let response = run(&filters, poll(&device_code)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["error"], "authorization_pending");

        // The resource owner enters the user code and approves the request.
        {
            let _resource_owner = SignedInResourceOwner::new();
            let response = run(
                &filters,
                token_request(
                    "/oauth2/device_verification",
                    &[("user_code", &user_code)],
                    None,
                ),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(json_body(response).await["status"], "approved");
        }

        // The device polls again and receives the tokens.
        let response = run(&filters, poll(&device_code)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert!(body["access_token"].as_str().is_some());
    }

    #[tokio::test]
    async fn the_device_grant_reports_a_denied_request() {
        let filters = security_filters();

        let response = run(
            &filters,
            token_request(
                "/oauth2/device_authorization",
                &[("scope", "read")],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;
        let body = json_body(response).await;
        let device_code = body["device_code"].as_str().expect("the code").to_string();
        let user_code = body["user_code"].as_str().expect("the code").to_string();

        {
            let _resource_owner = SignedInResourceOwner::new();
            let response = run(
                &filters,
                token_request(
                    "/oauth2/device_verification",
                    &[("user_code", &user_code), ("deny", "true")],
                    None,
                ),
            )
            .await;
            assert_eq!(json_body(response).await["status"], "denied");
        }

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", &device_code),
                ],
                Some((DEMO_CLIENT_ID, DEMO_CLIENT_SECRET)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_body(response).await["error"], "access_denied");
    }

    #[tokio::test]
    async fn the_registration_endpoint_registers_a_client() {
        let filters = security_filters();

        let mut request = Request::builder()
            .method("POST")
            .uri("/connect/register")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {DEMO_INITIAL_ACCESS_TOKEN}"),
            )
            .body(Body::empty())
            .expect("the request is built");
        buffer_request_body(
            &mut request,
            serde_json::json!({
                "client_name": "Registered Client",
                "grant_types": ["client_credentials"],
                "scope": "read",
            })
            .to_string(),
        );

        let response = run(&filters, request).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = json_body(response).await;
        let client_id = body["client_id"]
            .as_str()
            .expect("the client id")
            .to_string();
        let client_secret = body["client_secret"]
            .as_str()
            .expect("the client secret")
            .to_string();
        assert_eq!(body["client_secret_expires_at"], 0);

        // The registered client exchanges its credentials for a token.
        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[("grant_type", "client_credentials")],
                Some((&client_id, &client_secret)),
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(json_body(response).await["access_token"].as_str().is_some());
    }

    /// Builds a request with a JSON body for the client registration endpoint.
    ///
    /// # Arguments
    ///
    /// * `method` - The method of the request.
    /// * `path` - The path of the request.
    /// * `body` - The JSON body of the request.
    /// * `token` - The initial access token to present, if any.
    fn registration_request(
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
        token: Option<&str>,
    ) -> Request {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("Content-Type", "application/json");
        if let Some(token) = token {
            builder = builder.header("Authorization", format!("Bearer {token}"));
        }

        let mut request = builder.body(Body::empty()).expect("the request is built");
        if let Some(body) = body {
            buffer_request_body(&mut request, body.to_string());
        }

        request
    }

    #[tokio::test]
    async fn the_registration_endpoint_refuses_a_request_without_the_initial_access_token() {
        let filters = security_filters();

        let response = run(
            &filters,
            registration_request(
                "POST",
                "/connect/register",
                Some(serde_json::json!({
                    "client_name": "Open Client",
                    "grant_types": ["client_credentials"],
                    "scope": "read",
                })),
                None,
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(json_body(response).await["error"], "invalid_token");
    }

    #[tokio::test]
    async fn the_registration_endpoint_reads_updates_and_removes_a_client() {
        let filters = security_filters();

        let response = run(
            &filters,
            registration_request(
                "POST",
                "/connect/register",
                Some(serde_json::json!({
                    "client_name": "Managed Client",
                    "grant_types": ["client_credentials"],
                    "scope": "read",
                })),
                Some(DEMO_INITIAL_ACCESS_TOKEN),
            ),
        )
        .await;
        let created = json_body(response).await;
        let client_id = created["client_id"]
            .as_str()
            .expect("the registration carries the client id")
            .to_string();
        let path = format!("/connect/register/{client_id}");

        // The registration is read back.
        let response = run(
            &filters,
            registration_request("GET", &path, None, Some(DEMO_INITIAL_ACCESS_TOKEN)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await["client_name"], "Managed Client");

        // The registration is replaced.
        let response = run(
            &filters,
            registration_request(
                "PUT",
                &path,
                Some(serde_json::json!({
                    "client_name": "Renamed Client",
                    "grant_types": ["client_credentials"],
                    "scope": "read",
                })),
                Some(DEMO_INITIAL_ACCESS_TOKEN),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await["client_name"], "Renamed Client");

        // The registration is removed, and the client it named is gone.
        let response = run(
            &filters,
            registration_request("DELETE", &path, None, Some(DEMO_INITIAL_ACCESS_TOKEN)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = run(
            &filters,
            registration_request("GET", &path, None, Some(DEMO_INITIAL_ACCESS_TOKEN)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_client_authenticates_with_a_signed_assertion() {
        let filters = security_filters();

        let assertion = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &serde_json::json!({
                "iss": ASSERTION_CLIENT_ID,
                "sub": ASSERTION_CLIENT_ID,
                "aud": DEMO_ISSUER,
                "jti": "the-assertion",
                "exp": 4_102_444_800_u64,
            }),
            &jsonwebtoken::EncodingKey::from_secret(ASSERTION_CLIENT_SECRET.as_bytes()),
        )
        .expect("the assertion is signed");

        let response = run(
            &filters,
            token_request(
                "/oauth2/token",
                &[
                    ("grant_type", "client_credentials"),
                    (
                        "client_assertion_type",
                        "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
                    ),
                    ("client_assertion", &assertion),
                    ("client_id", ASSERTION_CLIENT_ID),
                ],
                None,
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(json_body(response).await["access_token"].as_str().is_some());
    }

    #[tokio::test]
    async fn the_remote_jwk_set_of_the_server_verifies_its_tokens() {
        let filters = security_filters();
        let base = serve(filters).await;
        let access_token = fetch_demo_token(&base).await;

        // A resource server reads the keys of the authorization server over
        // HTTP and verifies the token with them.
        let decoder = NimbusJwtDecoder::from_jwk_set_uri(format!("{base}/oauth2/jwks"));
        let jwt = decoder
            .decode(&access_token)
            .await
            .expect("the token is verified with the remote keys");

        assert_eq!(jwt.claim("iss").map(String::as_str), Some(DEMO_ISSUER));
        assert_eq!(jwt.claim("scope").map(String::as_str), Some("read"));

        // A token that was changed after it was signed is refused.
        let (head, _) = access_token
            .rsplit_once('.')
            .expect("the token has a signature");
        assert!(decoder.decode(&format!("{head}.AAAA")).await.is_err());
    }

    #[tokio::test]
    async fn the_opaque_token_introspector_reads_the_introspection_endpoint() {
        use next_web_security::oauth2::server::resource::SpringOpaqueTokenIntrospector;

        let filters = security_filters();
        let base = serve(filters).await;
        let access_token = fetch_demo_token(&base).await;

        let introspector = SpringOpaqueTokenIntrospector::new(
            format!("{base}/oauth2/introspect"),
            DEMO_CLIENT_ID,
            DEMO_CLIENT_SECRET,
        );
        let principal = introspector
            .introspect(&access_token)
            .await
            .expect("the token is active");

        assert_eq!(principal.name().as_ref(), DEMO_CLIENT_ID);
        assert_eq!(
            principal.attributes().get("active").map(String::as_str),
            Some("true")
        );
    }
}

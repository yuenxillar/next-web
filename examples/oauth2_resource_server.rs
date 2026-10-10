//! Example that wires the OAuth 2.0 resource server of an application.
//!
//! The application turns on the resource server support with the JWT decoder of
//! the framework, so the routes under `/api` require a bearer token.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin oauth2_resource_server --features oauth2
//! ```
//!
//! The example accepts the unsigned demo tokens below, which only a resource
//! server that trusts unsigned tokens would accept:
//!
//! ```text
//! read scope:  eyJhbGciOiJub25lIn0.eyJzdWIiOiJkZW1vIiwic2NvcGUiOiJyZWFkIn0.sig
//! admin scope: eyJhbGciOiJub25lIn0.eyJzdWIiOiJkZW1vIiwic2NvcGUiOiJhZG1pbiJ9.sig
//! ```
//!
//! With the application running:
//!
//! ```text
//! curl -i http://127.0.0.1:11000/app/api/me
//! curl -i -H "Authorization: Bearer <read token>" http://127.0.0.1:11000/app/api/me
//! curl -i -H "Authorization: Bearer <admin token>" http://127.0.0.1:11000/app/api/admin/report
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
    config::web::{builders::HttpSecurity, WebSecurityConfigurer},
    oauth2::server::resource::jwt::NimbusJwtDecoder,
};

/// An unsigned demo token that carries the `read` scope.
pub const DEMO_READ_TOKEN: &str = "eyJhbGciOiJub25lIn0.eyJzdWIiOiJkZW1vIiwic2NvcGUiOiJyZWFkIn0.sig";

/// An unsigned demo token that carries the `admin` scope.
pub const DEMO_ADMIN_TOKEN: &str =
    "eyJhbGciOiJub25lIn0.eyJzdWIiOiJkZW1vIiwic2NvcGUiOiJhZG1pbiJ9.sig";

/// An unsigned demo token that carries neither the `read` nor the `admin` scope.
pub const DEMO_WRITE_TOKEN: &str =
    "eyJhbGciOiJub25lIn0.eyJzdWIiOiJkZW1vIiwic2NvcGUiOiJ3cml0ZSJ9.sig";

/// The application of the example.
#[derive(Default)]
struct OAuth2ResourceServerApplication;

impl Application for OAuth2ResourceServerApplication {}

/// Contributes the event publisher the security configuration resolves.
#[derive(Clone)]
struct OAuth2ResourceServerAutoConfig;

#[configuration]
impl OAuth2ResourceServerAutoConfig {
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
pub struct OAuth2ResourceServerRouter;

impl OAuth2ResourceServerRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl Ordered for OAuth2ResourceServerRouter {
    fn order(&self) -> i32 {
        0
    }
}

impl ApplyRouter for OAuth2ResourceServerRouter {
    /// Builds the router of the example.
    ///
    /// The security filter chain is not built here: the auto-configuration of
    /// the framework builds it from the HTTP security and the web security
    /// configurers of the application, and the web layer installs the filter
    /// chain proxy in front of these routes.
    ///
    /// * `_ctx` - The context of the application.
    fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/public/hello", get(|| async { "public hello" }))
            .route("/api/me", get(|| async { "authenticated hello" }))
            .route("/api/admin/report", get(|| async { "admin report" }))
    }
}

/// Configures the OAuth 2.0 resource server of the example.
#[singleton(binds = [Self::into_web_security_configure])]
#[derive(Clone)]
struct OAuth2ResourceServerWebSecurityConfigurer;

impl OAuth2ResourceServerWebSecurityConfigurer {
    fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigurer> {
        Box::new(self)
    }
}

impl WebSecurityConfigurer for OAuth2ResourceServerWebSecurityConfigurer {
    fn configure(&mut self, http: &mut HttpSecurity) {
        configure_oauth2_resource_server(http);
    }
}

/// Applies the OAuth 2.0 resource server configuration of the example.
///
/// The application and the tests share the function, so the tests cover the
/// rules the example ships.
///
/// # Arguments
///
/// * `http` - The HTTP security the configuration is applied to.
pub fn configure_oauth2_resource_server(http: &mut HttpSecurity) {
    http.authorize_http_requests(|auth| {
        auth.request_matchers(&["/public/**"])
            .permit_all()
            .request_matchers(&["/api/admin/**"])
            .has_authority("SCOPE_admin")
            .any_request()
            .authenticated();
    })
    .oauth2_resource_server(|resource_server| {
        resource_server.jwt(|jwt| {
            jwt.decoder(Arc::new(NimbusJwtDecoder::default()));
        });
    });
}

#[tokio::main]
async fn main() {
    NextWebApplication::<OAuth2ResourceServerApplication>::default()
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
        traits::{
            filter::{HttpFilter, HttpFilterChain},
            http::{http_request::HttpRequest, http_response::HttpResponse},
        },
    };
    use next_web_security::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder, web::builders::HttpSecurity,
        },
        core::{AuthenticationError, AuthenticationErrorKind},
        oauth2::server::resource::{OAuth2AuthenticatedPrincipal, OpaqueTokenIntrospector},
        web::security_filter_chain::SecurityFilterChain,
    };

    use super::{
        configure_oauth2_resource_server, DEMO_ADMIN_TOKEN, DEMO_READ_TOKEN, DEMO_WRITE_TOKEN,
    };

    /// The opaque token the example introspection accepts.
    const DEMO_OPAQUE_TOKEN: &str = "demo-opaque-token";

    /// An introspector that accepts the demo token and rejects every other one.
    #[derive(Clone, Debug, Default)]
    struct DemoOpaqueTokenIntrospector;

    impl OpaqueTokenIntrospector for DemoOpaqueTokenIntrospector {
        fn introspect(
            &self,
            token: &str,
        ) -> Result<OAuth2AuthenticatedPrincipal, AuthenticationError> {
            if token != DEMO_OPAQUE_TOKEN {
                return Err(AuthenticationError::with_kind(
                    "The opaque token is not active",
                    AuthenticationErrorKind::BadCredentials,
                ));
            }

            Ok(OAuth2AuthenticatedPrincipal::new(
                "demo",
                Vec::new(),
                HashMap::from([("active".to_string(), "true".to_string())]),
            ))
        }
    }

    /// Builds the filters of the example the way a real application gets them.
    ///
    /// The rules are the rules the example declares, and the chain is built by
    /// the same HTTP security builder the auto-configuration of the framework
    /// uses, so the test covers the configuration the example ships.
    ///
    /// # Returns
    ///
    /// The filters of the chain, in the order they run.
    fn jwt_security_filters() -> Arc<[Arc<dyn HttpFilter>]> {
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
        default_configurers(&mut http);
        configure_oauth2_resource_server(&mut http);

        Arc::from(http.build().get_filters().to_vec())
    }

    /// Builds the filters of a resource server that introspects opaque tokens.
    ///
    /// # Returns
    ///
    /// The filters of the chain, in the order they run.
    fn opaque_security_filters() -> Arc<[Arc<dyn HttpFilter>]> {
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
        default_configurers(&mut http);
        http.authorize_http_requests(|auth| {
            auth.request_matchers(&["/public/**"])
                .permit_all()
                .any_request()
                .authenticated();
        })
        .oauth2_resource_server(|resource_server| {
            resource_server.opaque_token(|opaque| {
                opaque.introspector(Arc::new(DemoOpaqueTokenIntrospector));
            });
        });

        Arc::from(http.build().get_filters().to_vec())
    }

    /// Applies the configurers every application gets by default.
    ///
    /// # Arguments
    ///
    /// * `http` - The HTTP security the configurers are applied to.
    fn default_configurers(http: &mut HttpSecurity) {
        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .security_context(|_| {})
            .request_cache(|_| {})
            .anonymous(|_| {});
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
    /// The response the filters produced. A response that is not committed
    /// reached the end of the chain, which stands for the routes of the
    /// application.
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

    /// Builds a request for a path, optionally presenting a bearer token.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the request.
    /// * `token` - The bearer token to present, if any.
    ///
    /// # Returns
    ///
    /// The request.
    fn request(path: &str, token: Option<&str>) -> Request {
        let mut builder = Request::builder().uri(path);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        builder.body(Body::empty()).expect("the request is built")
    }

    #[tokio::test]
    async fn the_resource_server_lets_a_public_request_continue() {
        let filters = jwt_security_filters();

        let response = run(&filters, request("/public/hello", None)).await;

        assert!(!response.is_committed());
    }

    #[tokio::test]
    async fn the_resource_server_authenticates_a_jwt() {
        let filters = jwt_security_filters();

        let response = run(&filters, request("/api/me", Some(DEMO_READ_TOKEN))).await;

        assert!(!response.is_committed());
    }

    #[tokio::test]
    async fn the_resource_server_rejects_a_missing_bearer_token() {
        let filters = jwt_security_filters();

        let response = run(&filters, request("/api/me", None)).await;

        assert!(
            response.status() == StatusCode::UNAUTHORIZED
                || response.status() == StatusCode::FORBIDDEN,
            "unexpected status: {}",
            response.status()
        );
    }

    #[tokio::test]
    async fn the_resource_server_rejects_an_invalid_bearer_token() {
        let filters = jwt_security_filters();

        let response = run(&filters, request("/api/me", Some("not-a-jwt"))).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn the_resource_server_denies_a_token_without_the_authority() {
        let filters = jwt_security_filters();

        let response = run(
            &filters,
            request("/api/admin/report", Some(DEMO_WRITE_TOKEN)),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn the_resource_server_allows_a_token_with_the_authority() {
        let filters = jwt_security_filters();

        let response = run(
            &filters,
            request("/api/admin/report", Some(DEMO_ADMIN_TOKEN)),
        )
        .await;

        assert!(!response.is_committed());
    }

    #[tokio::test]
    async fn the_jwt_decoder_decodes_the_demo_token() {
        use next_web_security::oauth2::server::resource::jwt::{JwtDecoder, NimbusJwtDecoder};

        let jwt = NimbusJwtDecoder::default()
            .decode(DEMO_READ_TOKEN)
            .expect("the demo token decodes");

        assert_eq!(jwt.claim("sub").map(String::as_str), Some("demo"));
        assert_eq!(jwt.claim("scope").map(String::as_str), Some("read"));
    }

    #[tokio::test]
    async fn the_jwt_provider_authenticates_the_demo_token() {
        use std::any::TypeId;

        use next_web_security::{
            authentication::AuthenticationProvider,
            core::Authentication,
            oauth2::server::resource::{
                bearer::BearerTokenAuthenticationToken,
                jwt::{JwtAuthenticationProvider, NimbusJwtDecoder},
            },
        };

        let provider = JwtAuthenticationProvider::with_default_converter(Arc::new(
            NimbusJwtDecoder::default(),
        ));
        assert!(provider.supports(TypeId::of::<BearerTokenAuthenticationToken>()));

        let token: Arc<dyn Authentication> = Arc::new(
            BearerTokenAuthenticationToken::unauthenticated(DEMO_READ_TOKEN),
        );
        let authenticated = provider
            .authenticate(&token)
            .await
            .expect("the provider handles the token")
            .expect("the token is authenticated");

        assert!(authenticated.is_authenticated());
        assert_eq!(
            authenticated
                .authorities()
                .iter()
                .filter_map(|authority| authority.authority())
                .collect::<Vec<_>>(),
            vec!["SCOPE_read"]
        );
    }

    #[tokio::test]
    async fn the_bearer_converter_reads_the_authorization_header() {
        use std::any::TypeId;

        use next_web_security::{
            oauth2::server::resource::bearer::{
                BearerTokenAuthenticationConverter, BearerTokenAuthenticationToken,
            },
            web::authentication::AuthenticationConverter,
        };

        let converter = BearerTokenAuthenticationConverter::with_default_resolver();
        let mut request = request("/api/me", Some(DEMO_READ_TOKEN));
        request.ready();

        let converted = converter
            .convert(&mut request)
            .expect("the converter handles the request")
            .expect("the bearer token is converted");

        assert_eq!(
            converted.of(),
            TypeId::of::<BearerTokenAuthenticationToken>()
        );
    }

    #[tokio::test]
    async fn the_opaque_token_introspector_authenticates_a_token() {
        let filters = opaque_security_filters();

        let response = run(&filters, request("/api/me", Some(DEMO_OPAQUE_TOKEN))).await;

        assert!(!response.is_committed());
    }

    #[tokio::test]
    async fn the_opaque_token_introspector_rejects_an_unknown_token() {
        let filters = opaque_security_filters();

        let response = run(&filters, request("/api/me", Some("unknown"))).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

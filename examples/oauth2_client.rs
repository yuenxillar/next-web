//! Example that wires the OAuth 2.0 client of an application.
//!
//! The application declares one client registration and turns on OAuth 2.0
//! login. A request that starts the authorization code grant is redirected to
//! the authorization endpoint of the provider, and the redirect back from the
//! provider completes the login.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin oauth2_client --features oauth2
//! ```
//!
//! With the application running, the request that starts the grant answers with
//! the redirect to the provider:
//!
//! ```text
//! curl -i http://127.0.0.1:11000/app/oauth2/authorization/demo
//! ```

use std::sync::{Arc, Mutex};

use axum::routing::get;
use next_web::{
    Application, NextWebApplication,
    core::{ApplicationContext, Ordered, traits::apply_router::ApplyRouter},
    macros::{autoconfigure::configuration, bind::singleton},
};
use next_web_context::{ApplicationEvent, ApplicationEventPublisher};
use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use next_web_security::{
    config::web::{WebSecurityConfigurer, builders::HttpSecurity},
    oauth2::{
        AuthorizationRequestRepository, ClientRegistration, InMemoryClientRegistrationRepository,
        OAuth2AuthorizationRequest,
    },
};

/// The registration id the demo client is reached under.
pub const DEMO_REGISTRATION_ID: &str = "demo";

/// The client id the demo registration is configured with.
pub const DEMO_CLIENT_ID: &str = "demo-client";

/// The application of the example.
#[derive(Default)]
struct OAuth2ClientApplication;

impl Application for OAuth2ClientApplication {}

/// Contributes the event publisher the security configuration resolves.
#[derive(Clone)]
struct OAuth2ClientAutoConfig;

#[configuration]
impl OAuth2ClientAutoConfig {
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

/// The client registration the example redirects with.
///
/// # Returns
///
/// The registration of the demo provider.
pub fn demo_client_registration() -> ClientRegistration {
    ClientRegistration::new(
        DEMO_REGISTRATION_ID,
        "Demo Provider",
        "https://provider.example/authorize",
    )
    .set_client_id(DEMO_CLIENT_ID)
    .set_redirect_uri("http://127.0.0.1:11000/login/oauth2/code/demo")
    .set_scopes(["read", "admin"])
}

/// Keeps the authorization request in memory instead of in the session.
///
/// An application keeps the authorization request in the session, which is what
/// the default repository does. The example keeps it in memory so that the two
/// requests of the grant can be driven one after the other without a session.
#[derive(Clone, Default)]
pub struct InMemoryAuthorizationRequestRepository {
    authorization_request: Arc<Mutex<Option<OAuth2AuthorizationRequest>>>,
}

impl AuthorizationRequestRepository for InMemoryAuthorizationRequestRepository {
    fn load_authorization_request(
        &self,
        _request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        self.authorization_request
            .lock()
            .expect("the repository lock is not poisoned")
            .clone()
    }

    fn save_authorization_request(
        &self,
        authorization_request: Option<OAuth2AuthorizationRequest>,
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
        *self
            .authorization_request
            .lock()
            .expect("the repository lock is not poisoned") = authorization_request;
    }

    fn remove_authorization_request(
        &self,
        _request: &mut dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        self.authorization_request
            .lock()
            .expect("the repository lock is not poisoned")
            .take()
    }
}

/// Contributes the router of the example.
#[singleton(binds = [Self::into_apply_router])]
#[derive(Clone)]
pub struct OAuth2ClientRouter;

impl OAuth2ClientRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl Ordered for OAuth2ClientRouter {
    fn order(&self) -> i32 {
        0
    }
}

impl ApplyRouter for OAuth2ClientRouter {
    /// Builds the router of the example.
    ///
    /// The security filter chain is not built here: the auto-configuration of
    /// the framework builds it from the HTTP security and the web security
    /// configurers of the application, and the web layer installs the filter
    /// chain proxy in front of these routes.
    ///
    /// * `_ctx` - The context of the application.
    fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> axum::Router {
        axum::Router::new().route("/public/hello", get(|| async { "public hello" }))
    }
}

/// Configures the OAuth 2.0 client of the example.
#[singleton(binds = [Self::into_web_security_configure])]
#[derive(Clone)]
struct OAuth2ClientWebSecurityConfigurer;

impl OAuth2ClientWebSecurityConfigurer {
    fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigurer> {
        Box::new(self)
    }
}

impl WebSecurityConfigurer for OAuth2ClientWebSecurityConfigurer {
    fn configure(&mut self, http: &mut HttpSecurity) {
        // The application keeps the authorization request in the session, which
        // is the default repository of the framework.
        configure_oauth2_client(http, None);
    }
}

/// Applies the OAuth 2.0 client configuration of the example.
///
/// The application and the tests share the function, so the tests cover the
/// rules the example ships.
///
/// # Arguments
///
/// * `http` - The HTTP security the configuration is applied to.
/// * `authorization_request_repository` - The repository the authorization
///   request is kept in. The default, session based repository is used when
///   none is given.
pub fn configure_oauth2_client(
    http: &mut HttpSecurity,
    authorization_request_repository: Option<Arc<dyn AuthorizationRequestRepository>>,
) {
    // The application owns its registrations, and hands them to the login
    // configurer, which is what resolves the registration that the
    // authorization request names.
    let client_registration_repository = Arc::new(InMemoryClientRegistrationRepository::new([
        demo_client_registration(),
    ]));

    http.authorize_http_requests(|auth| {
        auth.request_matchers(&["/public/**"])
            .permit_all()
            .any_request()
            .authenticated();
    })
    .oauth2_login(|login| {
        login.client_registration_repository(client_registration_repository.clone());
        if let Some(authorization_request_repository) = authorization_request_repository.clone() {
            login.authorization_endpoint(|endpoint| {
                endpoint.authorization_request_repository(authorization_request_repository);
            });
        }
    });
}

#[tokio::main]
async fn main() {
    NextWebApplication::<OAuth2ClientApplication>::default()
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
        oauth2::AuthorizationRequestRepository,
        web::security_filter_chain::SecurityFilterChain,
    };

    use super::{
        DEMO_CLIENT_ID, DEMO_REGISTRATION_ID, InMemoryAuthorizationRequestRepository,
        configure_oauth2_client,
    };

    /// Builds the filters of the example the way a real application gets them.
    ///
    /// The rules are the rules the example declares, and the chain is built by
    /// the same HTTP security builder the auto-configuration of the framework
    /// uses, so the test covers the configuration the example ships.
    ///
    /// # Arguments
    ///
    /// * `authorization_request_repository` - The repository the authorization
    ///   request is kept in.
    ///
    /// # Returns
    ///
    /// The filters of the chain, in the order they run.
    fn security_filters(
        authorization_request_repository: Option<Arc<dyn AuthorizationRequestRepository>>,
    ) -> Arc<[Arc<dyn HttpFilter>]> {
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .security_context(|_| {})
            .request_cache(|_| {})
            .anonymous(|_| {});
        configure_oauth2_client(&mut http, authorization_request_repository);

        let filter_chain = http.build();
        Arc::from(filter_chain.get_filters().to_vec())
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

    /// Builds a request for a path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the request.
    ///
    /// # Returns
    ///
    /// The request.
    fn request(path: &str) -> Request {
        Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("the request is built")
    }

    /// Returns the location of a redirect response.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    ///
    /// # Returns
    ///
    /// The location the response redirects to.
    fn redirect_location(response: &Response) -> String {
        response
            .headers()
            .get(header::LOCATION)
            .and_then(|location| location.to_str().ok())
            .expect("the response redirects")
            .to_string()
    }

    /// Returns the value of a query parameter of a redirect location.
    ///
    /// # Arguments
    ///
    /// * `location` - The location the redirect names.
    /// * `name` - The name of the parameter.
    ///
    /// # Returns
    ///
    /// The value of the parameter, if the location carries it.
    fn redirect_parameter(location: &str, name: &str) -> Option<String> {
        let query = location.split_once('?')?.1;
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key == name).then(|| value.to_string())
        })
    }

    #[tokio::test]
    async fn the_client_starts_the_authorization_code_grant() {
        let filters = security_filters(None);

        let response = run(
            &filters,
            request(&format!("/oauth2/authorization/{DEMO_REGISTRATION_ID}")),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FOUND);
        let location = redirect_location(&response);
        assert!(
            location.starts_with("https://provider.example/authorize?"),
            "unexpected authorization endpoint: {location}"
        );
        assert!(location.contains("response_type=code"), "{location}");
        assert!(
            location.contains(&format!("client_id={DEMO_CLIENT_ID}")),
            "{location}"
        );
        assert!(location.contains("state="), "{location}");
    }

    #[tokio::test]
    async fn the_client_completes_the_authorization_code_grant() {
        let repository = InMemoryAuthorizationRequestRepository::default();
        let filters = security_filters(Some(Arc::new(repository.clone())));

        // The first request starts the grant and redirects to the
        // authorization endpoint of the provider.
        let redirect = run(
            &filters,
            request(&format!("/oauth2/authorization/{DEMO_REGISTRATION_ID}")),
        )
        .await;
        assert_eq!(redirect.status(), StatusCode::FOUND);
        let state = redirect_parameter(&redirect_location(&redirect), "state")
            .expect("the redirect carries the state of the authorization request");
        assert!(
            repository
                .authorization_request
                .lock()
                .expect("the repository lock is not poisoned")
                .is_some(),
            "the authorization request is kept until the login completes"
        );

        // The second request is the redirect back from the provider. The token
        // endpoint of the example echoes the authorization code, so the login
        // succeeds and the filter redirects the client to the application.
        let callback =
            format!("/login/oauth2/code/{DEMO_REGISTRATION_ID}?code=accepted&state={state}");
        let response = run(&filters, request(&callback)).await;

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(redirect_location(&response), "/");
        assert!(
            repository
                .authorization_request
                .lock()
                .expect("the repository lock is not poisoned")
                .is_none(),
            "the authorization request is consumed by the login"
        );
    }

    #[tokio::test]
    async fn the_client_rejects_an_unknown_registration() {
        let filters = security_filters(None);

        let response = run(&filters, request("/oauth2/authorization/missing")).await;

        // The resolver does not know the registration, so the request is not
        // redirected to a provider: the failure handler of the filter reports
        // the failure instead of sending the client to a provider that cannot
        // accept it.
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn the_client_lets_an_unrelated_request_continue() {
        let filters = security_filters(None);

        let response = run(&filters, request("/public/hello")).await;

        assert!(!response.is_committed());
    }
}

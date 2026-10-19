//! The security example: requests that are already authenticated when they arrive.
//!
//! The deployment authenticates the user before the request reaches the
//! application: a reverse proxy, a gateway or the identity provider of a single
//! sign-on writes the name of the user it authenticated into the `X-User` header.
//! The filter this example configures reads that header and turns it into an
//! authentication, so no handler of the example ever asks for a password.
//!
//! The handlers carry their rules themselves ([`pre_authorize`]), so the
//! configuration of the chain only says who the users are and what a refused
//! request answers.
//!
//! Try it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin security
//! curl http://127.0.0.1:11000/app/auth/hello                          # 200
//! curl http://127.0.0.1:11000/app/auth/user                          # 403
//! curl -H "X-User: user" http://127.0.0.1:11000/app/auth/user        # 200
//! curl -H "X-User: user" http://127.0.0.1:11000/app/auth/orders      # 200
//! curl -H "X-User: other" http://127.0.0.1:11000/app/auth/orders     # 403
//! ```

use std::sync::Arc;

use axum::http::StatusCode;
use next_web::{
    Application, NextWebApplication,
    macros::{
        autoconfigure::configuration,
        bind::{get_mapping, singleton},
        security::pre_authorize,
    },
};
use next_web_context::{ApplicationEvent, ApplicationEventPublisher};
use next_web_core::error::BoxError;
use next_web_security::{
    authentication::provider_manager::ProviderManager,
    config::{
        http::SessionCreationPolicy,
        web::{HttpSecurityBuilder, WebSecurityConfigurer, builders::HttpSecurity},
    },
    core::userdetails::{User, UserDetailsByNameServiceWrapper, UserDetailsService},
    provisioning::InMemoryUserDetailsManager,
    web::{
        access::access_denied_handler_fn_wrapper,
        authentication::{
            AnonymousAuthenticationFilter,
            preauth::{
                PreAuthenticatedAuthenticationProvider, PreAuthenticatedAuthenticationToken,
                request_header_authentication_filter::RequestHeaderAuthenticationFilter,
            },
        },
        context::RequestAttributeSecurityContextRepository,
    },
};

/// The header the deployment writes the authenticated user into.
const PRINCIPAL_HEADER: &str = "X-User";

#[derive(Default)]
struct SecurityApplication;

impl Application for SecurityApplication {}

/// Answers every request, whether it carries an authentication or not.
#[pre_authorize("permitAll()")]
#[get_mapping(path = "/auth/hello")]
async fn hello() -> &'static str {
    "Hello!"
}

/// Answers a request that carries the authority of the example.
#[pre_authorize("has_authority('order:read')")]
#[get_mapping(path = "/auth/get")]
async fn authorized() -> &'static str {
    "Authorized"
}

/// Answers a request that was authenticated with the `USER` role.
#[pre_authorize("has_role('USER')")]
#[get_mapping(path = "/auth/user")]
async fn user_hello() -> &'static str {
    "Hello, user!"
}

/// Answers a request that may read the orders and does not carry the banned
/// role.
#[pre_authorize("has_authority('order:read') and !has_role('BANNED')")]
#[get_mapping(path = "/auth/orders")]
async fn order_reads() -> &'static str {
    "the orders"
}

/// The structured spelling of a rule: an auditor, or a reader of the audit.
#[pre_authorize(has_role = "AUDITOR", has_authority = "audit:read", mode = "Or")]
#[get_mapping(path = "/auth/audit")]
async fn audit() -> &'static str {
    "the audit"
}

/// Configures the requests of the example as pre-authenticated requests.
///
/// The users of the example are held in memory, because who they are is not what
/// this example is about: the point is that the name arrives in a header and the
/// authorities the rules read come from that user, without a login form, without
/// a password and without a session.
#[singleton(binds = [Self::into_web_security_configure])]
#[derive(Clone)]
struct PreAuthenticatedRequests;

impl PreAuthenticatedRequests {
    fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigurer> {
        Box::new(self)
    }
}

impl WebSecurityConfigurer for PreAuthenticatedRequests {
    fn configure(&mut self, http: &mut HttpSecurity) {
        // The user the header names, and what that user may do.
        let user_details_service: Arc<dyn UserDetailsService> =
            Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
                User::with_username("user")
                    .authorities_with_strings(&["ROLE_USER", "order:read"])
                    .build(),
            )]));

        // The provider looks the name of the header up and gives the request the
        // authorities of that user.
        let mut provider = PreAuthenticatedAuthenticationProvider::default();
        provider.set_pre_authenticated_user_details_service(Arc::new(
            UserDetailsByNameServiceWrapper::<PreAuthenticatedAuthenticationToken>::new(
                user_details_service,
            ),
        ));

        // The filter reads the header of every request. A request without it, or
        // with a name the example does not know, is left without an authentication:
        // the handlers that permit everyone answer it, and the ones that demand an
        // authority refuse it.
        let mut filter = RequestHeaderAuthenticationFilter::default();
        filter.set_principal_request_header(PRINCIPAL_HEADER);
        filter.set_error_if_header_missing(false);
        filter.set_authentication_manager(Arc::new(ProviderManager::new(vec![Arc::new(provider)])));
        // The request carries its own context: the example is stateless, so the
        // context is not written into a session that the next request would read.
        filter.set_security_context_repository(Arc::new(
            RequestAttributeSecurityContextRepository::default(),
        ));
        http.add_filter_before::<RequestHeaderAuthenticationFilter, AnonymousAuthenticationFilter>(
            filter,
        );

        // The handlers carry their own rules, so the chain itself lets every
        // request through: a request is refused by the rule of the handler it is
        // made for, not by the chain.
        http.authorize_http_requests(|requests| {
            requests.any_request().permit_all();
        })
        .session_management(|session| {
            session.session_creation_policy(SessionCreationPolicy::Stateless);
        })
        .error_handling(|error_handling| {
            error_handling.access_denied_handler(access_denied_handler_fn_wrapper(
                |_request, response, _error| {
                    response.insert_header("Content-Type", "application/json;charset=UTF-8");
                    response.set_status_code(StatusCode::FORBIDDEN);
                    response.set_body(
                        r#"{"code": 403, "message": "Insufficient permissions"}"#
                            .as_bytes()
                            .to_vec(),
                    );

                    Ok(())
                },
            ));
        });
    }
}

#[derive(Clone)]
struct SecurityAutoConfig {}

#[configuration]
impl SecurityAutoConfig {
    /// Publishes the events of the application.
    ///
    /// The security configuration publishes the events of an authentication
    /// through the publisher of the application, so an application that runs
    /// without a publisher of its own provides the default one here.
    #[provider(name = "applicationEventPublisher")]
    fn application_event_publisher() -> Arc<dyn ApplicationEventPublisher> {
        #[derive(Clone)]
        struct DefaultApplicationEventPublisher;

        impl ApplicationEventPublisher for DefaultApplicationEventPublisher {
            fn publish_event(&self, _event: Box<dyn ApplicationEvent>) -> Result<(), BoxError> {
                Ok(())
            }
        }

        Arc::new(DefaultApplicationEventPublisher)
    }


}

#[tokio::main]
async fn main() {
    NextWebApplication::<SecurityApplication>::default().run().await
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, collections::HashMap, sync::Arc};

    use axum::{
        body::Body,
        http::Request,
        response::Response,
    };
    use next_web_core::{
        any::AnyClone,
        async_trait,
        http::{
            HttpRequest, HttpResponse,
            filter::{ApplicationFilterChain, FilterError, HttpFilter, HttpFilterChain},
        },
        Named,
    };
    use next_web_security::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder,
            web::{WebSecurityConfigurer, builders::HttpSecurity},
        },
        core::context::SecurityContextHolder,
        web::{
            security_filter_chain::SecurityFilterChain,
            util::matcher::PathPatternRequestMatcherBuilder,
        },
    };

    use super::PreAuthenticatedRequests;

    /// The terminal of the chains of the tests: it records who the request was
    /// made for, which is what the filter of the example established.
    #[derive(Clone)]
    struct RecordTheUser;

    #[async_trait]
    impl HttpFilter for RecordTheUser {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            response: &mut dyn HttpResponse,
            _filter_chain: &dyn HttpFilterChain,
        ) -> Result<(), FilterError> {
            let recorded = SecurityContextHolder::get_context()
                .get_authentication()
                .map(|authentication| {
                    let authorities = authentication
                        .authorities()
                        .iter()
                        .map(|authority| {
                            authority.authority().unwrap_or_default().to_string()
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("{} [{authorities}]", authentication.name())
                })
                .unwrap_or_else(|| "no authentication".to_string());
            response.set_body(recorded.into_bytes());

            Ok(())
        }

        fn supports(&self, name: &str) -> bool {
            name == "RecordTheUser"
        }
    }

    impl Named for RecordTheUser {
        fn name(&self) -> &str {
            "RecordTheUser"
        }
    }

    /// Returns the filters of the example, in the order they run.
    ///
    /// The chain holds the filters the framework applies to every application —
    /// the CSRF, the headers, the context and the anonymous fallback — and the
    /// ones the example configures.
    fn filters() -> Arc<[Arc<dyn HttpFilter>]> {
        let mut shared_objects: HashMap<TypeId, Box<dyn AnyClone>> = HashMap::new();
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(PathPatternRequestMatcherBuilder::default()),
        );
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .security_context(|_| {})
            .anonymous(|_| {});
        PreAuthenticatedRequests.configure(&mut http);

        let mut filters = http.build().get_filters().to_vec();
        filters.push(Arc::new(RecordTheUser) as Arc<dyn HttpFilter>);

        Arc::from(filters)
    }

    /// Returns the names of the filters of the example, in the order they run.
    fn filter_names() -> Vec<String> {
        filters()
            .iter()
            .map(|filter| filter.name().to_owned())
            .collect()
    }

    /// Runs one request through the filters of the example.
    async fn run(request: Request<Body>) -> Response {
        let chain = ApplicationFilterChain::new(filters());
        let mut request = request;
        request.ready();
        let mut response = Response::new(Body::empty());
        chain
            .do_filter(&mut request, &mut response)
            .await
            .expect("the filter chain handles the request");

        response
    }

    /// Returns the body of a response.
    async fn body_of(response: Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is collected");

        String::from_utf8_lossy(&body).to_string()
    }

    /// Returns a request for a path, made for the user named by the header.
    fn request(path: &str, user: Option<&str>) -> Request<Body> {
        let mut request = Request::builder().uri(path);
        if let Some(user) = user {
            request = request.header(super::PRINCIPAL_HEADER, user);
        }

        request.body(Body::empty()).expect("the request is built")
    }

    #[tokio::test]
    async fn the_user_of_the_header_is_authenticated_with_its_authorities() {
        let response = run(request("/auth/orders", Some("user"))).await;
        let recorded = body_of(response).await;

        assert_eq!(
            recorded, "user [ROLE_USER,order:read]",
            "the name of the header is the user, and the authorities of that user are the ones \
             the rules read"
        );
    }

    #[tokio::test]
    async fn a_request_without_the_header_is_left_without_an_authentication() {
        let response = run(request("/auth/hello", None)).await;
        let recorded = body_of(response).await;

        assert!(
            recorded.contains("ROLE_ANONYMOUS"),
            "a request the deployment did not authenticate is anonymous, not a user of the \
             example: {recorded}"
        );
        assert!(
            !recorded.contains("user"),
            "the request is not the user of the example: {recorded}"
        );
    }

    #[tokio::test]
    async fn a_user_the_example_does_not_know_is_left_without_an_authentication() {
        let response = run(request("/auth/orders", Some("other"))).await;
        let recorded = body_of(response).await;

        assert!(
            recorded.contains("ROLE_ANONYMOUS"),
            "a name the users of the example do not hold is not an authentication: {recorded}"
        );
        assert!(
            !recorded.contains("other"),
            "the name of the header is not taken as a user of the example: {recorded}"
        );
    }

    #[test]
    fn the_example_authenticates_a_request_before_the_anonymous_fallback() {
        let names = filter_names();

        let pre_authenticated = names
            .iter()
            .position(|name| name == "RequestHeaderAuthenticationFilter")
            .unwrap_or_else(|| panic!("the filter of the header is part of the chain: {names:?}"));
        let anonymous = names
            .iter()
            .position(|name| name == "AnonymousAuthenticationFilter")
            .expect("the anonymous fallback is part of the chain");
        assert!(
            pre_authenticated < anonymous,
            "the header is read before the anonymous fallback runs: {names:?}"
        );
    }

    #[test]
    fn the_example_holds_no_password_authentication() {
        let names = filter_names();

        assert!(
            !names.iter().any(|name| name == "UsernamePasswordAuthenticationFilter"
                || name == "BasicAuthenticationFilter"),
            "a pre-authenticated example asks for no password: {names:?}"
        );
    }
}

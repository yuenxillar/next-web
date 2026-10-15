use std::{fs::File, io::Read, sync::Arc};

use axum::{
    response::Html,
    routing::{get, post},
};
use next_web::{
    core::{traits::apply_router::ApplyRouter, ApplicationContext},
    macros::autoconfigure::configuration,
    macros::bind::singleton,
    macros::order,
    macros::security::pre_authorize,
    Application, NextWebApplication,
};
use next_web_context::{ApplicationEvent, ApplicationEventPublisher};
use next_web_core::error::BoxError;

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

/// Contributes the router of the security example.
#[order(0)]
#[singleton(binds = [Self::into_apply_router])]
#[derive(Clone)]
pub struct SecurityRouter;

impl SecurityRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl ApplyRouter for SecurityRouter {
    /// Builds the router of the security example.
    ///
    /// The security filter chain is not built here. The auto-configuration of
    /// the framework builds it from the HTTP security and the web security
    /// configurers of the application, and the web layer installs the filter
    /// chain proxy in front of these routes.
    ///
    /// * `_ctx` - The context of the application.
    fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route(
                "/login.jsp",
                get(|| async {
                    let mut fs = File::open(format!(
                        "{}/resources/login.html",
                        std::env::var("CARGO_MANIFEST_DIR").unwrap()
                    ))
                    .unwrap();
                    let mut buf = Vec::new();
                    fs.read_to_end(&mut buf).unwrap();
                    Html(buf)
                }),
            )
            .nest(
                "/auth",
                axum::Router::new()
                    .route("/hello", get(hello))
                    .route("/get", post(authorized))
                    .route("/user", get(user_hello))
                    .route("/orders", get(order_reads))
                    .route("/audit", get(audit)),
            )
    }
}

/// Answers every request, whether it carries an authentication or not.
#[pre_authorize("permitAll()")]
async fn hello() -> &'static str {
    "Hello!"
}

/// Answers a request that carries the authority of the example.
#[pre_authorize("has_authority('order:read')")]
async fn authorized() -> &'static str {
    "Authorized"
}

/// Answers a request that was authenticated with the `USER` role.
#[pre_authorize("has_role('USER')")]
async fn user_hello() -> &'static str {
    "Hello, user!"
}

/// Answers a request that may read the orders and does not carry the banned
/// role.
#[pre_authorize("has_authority('order:read') and !has_role('BANNED')")]
async fn order_reads() -> &'static str {
    "the orders"
}

/// The structured spelling of a rule: an auditor, or a reader of the audit.
#[pre_authorize(has_role = "AUDITOR", has_authority = "audit:read", mode = "Or")]
async fn audit() -> &'static str {
    "the audit"
}

#[derive(Clone)]
struct SecurityAutoConfig {}

#[configuration]
impl SecurityAutoConfig {
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

mod t2 {

    use std::sync::Arc;

    use axum::http::StatusCode;
    use next_web::macros::bind::singleton;
    use next_web_core::http::HttpMethod;
    use next_web_security::{
        authentication::dao::DaoAuthenticationProvider,
        config::{
            http::SessionCreationPolicy,
            web::{builders::HttpSecurity, HttpSecurityBuilder, WebSecurityConfigurer},
        },
        core::userdetails::User,
        crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
        provisioning::InMemoryUserDetailsManager,
        web::{access::access_denied_handler_fn_wrapper, authentication_entry_point_fn_wrapper},
    };

    #[singleton(binds = [Self::into_web_security_configure])]
    #[derive(Clone)]
    struct TestWebSecurityConfigure;

    impl TestWebSecurityConfigure {
        fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigurer> {
            Box::new(self)
        }
    }

    impl WebSecurityConfigurer for TestWebSecurityConfigure {
        fn configure(&mut self, http: &mut HttpSecurity) {
            // The resource owner of the example signs in with HTTP basic
            // authentication, so the guarded handlers of the example are
            // reached with an authentication their rules are evaluated against.
            let password = BCryptPasswordEncoder::default()
                .encode(Some("password"))
                .expect("the password of the example is encoded")
                .expect("the encoded password exists");
            let mut provider = DaoAuthenticationProvider::new(Arc::new(
                InMemoryUserDetailsManager::with_users(vec![Arc::new(
                    User::with_username("user")
                        .password(Some(password))
                        .roles(&["USER".to_string()])
                        .build(),
                )]),
            ));
            provider.set_password_encoder(Arc::new(BCryptPasswordEncoder::default()));
            http.authentication_provider(Arc::new(provider));

            http.csrf(|csrf| {
                csrf.get_security_context_holder_strategy();
            })
            .authorize_http_requests(|auth| {
                auth.request_matchers(&["/auth/**"])
                    .permit_all()
                    .request_matchers(HttpMethod::OPTIONS)
                    .has_authority("admin")
                    .any_request()
                    .authenticated();
            })
            .headers(|_| {})
            .error_handling(|eh| {
                eh.authentication_entry_point(authentication_entry_point_fn_wrapper(
                    |_req, resp, _err| {
                        resp.insert_header("Content-Type", "application/json;charset=UTF-8");
                        resp.set_status_code(StatusCode::UNAUTHORIZED);
                        resp.set_body(
                            r#"{"code": 401, "message": "Not authenticated, please log in first"}"#
                                .as_bytes()
                                .to_vec(),
                        );

                        Ok(())
                    },
                ))
                .access_denied_handler(access_denied_handler_fn_wrapper(|_req, resp, _err| {
                    resp.insert_header("Content-Type", "application/json;charset=UTF-8");
                    resp.set_status_code(StatusCode::FORBIDDEN);
                    resp.set_body(
                        r#"{"code": 403, "message": "Insufficient permissions"}"#
                            .as_bytes()
                            .to_vec(),
                    );

                    Ok(())
                }));
            })
            .security_context(|_| {})
            .anonymous(|_| {})
            .session_management(|session| {
                session.session_creation_policy(SessionCreationPolicy::Stateless);
            })
            .request_cache(|_| {})
            .logout(|_| {})
            .form_login(|_| {})
            .http_basic(|_| {
                // The resource owner of the example presents the credentials of
                // the user the example registered.
            });
        }
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, collections::HashMap, sync::Arc};

    use axum::{http::StatusCode, response::IntoResponse};
    use next_web_core::{any::AnyClone, security::PreAuthorizeContext};
    use next_web_security::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder, web::builders::HttpSecurity,
        },
        core::userdetails::{User, UserDetailsService},
        provisioning::InMemoryUserDetailsManager,
        web::security_filter_chain::SecurityFilterChain,
    };

    use super::{audit, authorized, hello, order_reads, user_hello};

    /// Builds an HTTP security that was not given an application context.
    ///
    /// The configurers that may take a value from the application context fall
    /// back to their own default instead of failing when the builder was not
    /// given one, and the user details service the one-time token login needs is
    /// shared the way the framework shares it.
    fn security_without_context() -> HttpSecurity {
        let user_details_service: Arc<dyn UserDetailsService> =
            Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
                User::with_username("user")
                    .password(Some("password".to_string()))
                    .build(),
            )]));
        let mut shared_objects: HashMap<TypeId, Box<dyn AnyClone>> = HashMap::new();
        shared_objects.insert(
            TypeId::of::<Arc<dyn UserDetailsService>>(),
            Box::new(user_details_service),
        );

        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .anonymous(|_| {})
            .one_time_token_login(|_| {})
            .saml2_logout(|_| {})
            .saml2_metadata(|_| {});

        http
    }

    #[test]
    fn a_builder_without_a_context_builds_the_login_features() {
        let chain = security_without_context().build();

        assert!(!chain.get_filters().is_empty());
    }

    /// Returns the guard of a request whose authorities are known.
    ///
    /// The guard of a handler is the parameter the `#[pre_authorize]` attribute
    /// reads the request with, so a test that calls a guarded handler names the
    /// authorities of the request itself.
    ///
    /// # Arguments
    ///
    /// * `authorities` - The authorities of the request.
    fn guard(authorities: &[&str]) -> PreAuthorizeContext {
        PreAuthorizeContext::for_authorities(authorities.to_vec())
    }

    /// Returns the status of the response of a handler.
    ///
    /// # Arguments
    ///
    /// * `response` - The response the handler produced.
    fn status(response: impl IntoResponse) -> StatusCode {
        response.into_response().status()
    }

    #[tokio::test]
    async fn a_handler_that_allows_every_request_answers_without_an_authentication() {
        assert_eq!(status(hello(guard(&[])).await), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_handler_that_requires_a_role_refuses_a_request_without_the_role() {
        assert_eq!(status(user_hello(guard(&[])).await), StatusCode::FORBIDDEN);
        assert_eq!(
            status(user_hello(guard(&["ROLE_ADMIN"])).await),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn a_handler_that_requires_a_role_answers_a_request_with_the_role() {
        assert_eq!(
            status(user_hello(guard(&["ROLE_USER"])).await),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn a_handler_that_requires_an_authority_answers_the_authority() {
        assert_eq!(
            status(authorized(guard(&["order:read"])).await),
            StatusCode::OK
        );
        assert_eq!(
            status(order_reads(guard(&["order:read"])).await),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn a_handler_that_requires_an_authority_refuses_another_request() {
        assert_eq!(
            status(authorized(guard(&["ROLE_USER"])).await),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            status(order_reads(guard(&["ROLE_USER"])).await),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn a_rule_that_negates_a_role_refuses_a_request_with_the_role() {
        assert_eq!(
            status(order_reads(guard(&["order:read", "ROLE_BANNED"])).await),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn a_structured_rule_with_or_answers_either_of_its_checks() {
        assert_eq!(status(audit(guard(&["audit:read"])).await), StatusCode::OK);
        assert_eq!(
            status(audit(guard(&["ROLE_AUDITOR"])).await),
            StatusCode::OK
        );
        assert_eq!(
            status(audit(guard(&["ROLE_USER"])).await),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn a_refused_request_is_rendered_by_the_handler_of_the_application() {
        use next_web_core::{
            error::BoxError,
            security::PreAuthorizeDenialHandler,
           http::{HttpRequest, HttpResponse},
        };

        struct JsonRefusal;

        impl PreAuthorizeDenialHandler for JsonRefusal {
            fn render(
                &self,
                _request: &mut dyn HttpRequest,
                response: &mut dyn HttpResponse,
            ) -> Result<(), BoxError> {
                response.insert_header("Content-Type", "application/json;charset=UTF-8");
                response.set_status_code(StatusCode::FORBIDDEN);
                response
                    .set_body(br#"{"code": 403, "message": "Insufficient permissions"}"#.to_vec());

                Ok(())
            }
        }

        // The guard carries the handler the application configured for its
        // filter chain, so a refused request is answered the way the filter
        // chain answers one.
        let response = user_hello(
            PreAuthorizeContext::for_authorities(Vec::<String>::new())
                .with_denial_handler(Arc::new(JsonRefusal)),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            response
                .headers()
                .get("Content-Type")
                .and_then(|content_type| content_type.to_str().ok()),
            Some("application/json;charset=UTF-8")
        );
    }
}

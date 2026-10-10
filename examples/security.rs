use std::{fs::File, io::Read, sync::Arc};

use axum::{
    response::Html,
    routing::{get, post},
};
use next_web::{
    Application, NextWebApplication,
    core::{ApplicationContext, Ordered, traits::apply_router::ApplyRouter},
    macros::autoconfigure::configuration,
    macros::bind::singleton,
};
use next_web_context::{ApplicationEvent, ApplicationEventPublisher};
use next_web_core::error::BoxError;

#[derive(Default)]
struct TestApplication;

impl Application for TestApplication {}

/// Contributes the router of the security example.
#[singleton(binds = [Self::into_apply_router])]
#[derive(Clone)]
pub struct SecurityRouter;

impl SecurityRouter {
    fn into_apply_router(self) -> Box<dyn ApplyRouter> {
        Box::new(self)
    }
}

impl Ordered for SecurityRouter {
    fn order(&self) -> i32 {
        0
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
                    .route("/hello", get(async || "Hello!"))
                    .route("/get", post(async || "Authorized")),
            )
    }
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

    use axum::http::StatusCode;
    use next_web::macros::bind::singleton;
    use next_web_core::http::HttpMethod;
    use next_web_security::{
        config::{
            http::SessionCreationPolicy,
            web::{WebSecurityConfigurer, builders::HttpSecurity},
        },
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
            .headers(|headers| {
                headers.get_security_context_holder_strategy();
            })
            .port_mapper(|pm| {
                pm.http(30).maps_to(1000).http(40).maps_to(1010);
            })
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
            .security_context(|sc| {
                sc.is_require_explicit_save();
            })
            .anonymous(|a| {
                a.get_security_context_holder_strategy();
            })
            .session_management(|session| {
                session.session_creation_policy(SessionCreationPolicy::Stateless);
            })
            .request_cache(|r| {
                r.get_security_context_holder_strategy();
            })
            .logout(|f| {
                f.get_security_context_holder_strategy();
            })
            .form_login(|f| {
                f.get_authentication_filter();
            });
        }
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

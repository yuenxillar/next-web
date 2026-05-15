use std::{fs::File, io::Read, sync::Arc};

use axum::{
    body::Body,
    extract::{Path, Request, State},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use next_web::{
    application::Application,
    core::{
        ApplicationContext, context::properties::ApplicationProperties,
        filter::application_filter_chain::ApplicationFilterChain,
    },
    macros::bind::singleton,
};
use next_web_core::async_trait;
use next_web_core::state::application_state::ApplicationState;
use next_web_core::traits::filter::http_filter::HttpFilter;
// use next_web_security::web::filter_proxy::FilterProxy;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route(
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
        // .nest(
        //     "/auth",
        //     axum::Router::new()
        //         .route("/hello", get(async || "Hello!"))
        //         .route("/setToken/{token}", post(set_token))
        //         .route("/get", post(async || "Authorized"))
        //         .route_layer(axum::middleware::from_fn_with_state(
        //             Arc::new(FilterProxy::default()),
        //             security_middleware,
        //         )),
        // )
    }

    async fn on_ready(
        &self,
        ctx: &mut ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(Vec::<String>::new())), "tokenStore");

        Ok(())
    }
}

mod t2 {
    use next_web::macros::bind::singleton;
    use next_web_core::util::http_method::HttpMethod;
    use next_web_security::{
        config::web::{
            configurers::{CsrfConfigurer, base_http_configurer::BaseHttpConfigurer},
            http_security::HttpSecurity,
        },
        core::web_security_configure::WebSecurityConfigure,
    };

    #[singleton(binds = [Self::into_web_security_configure])]
    #[derive(Clone)]
    struct TestWebSecurityConfigure;

    impl TestWebSecurityConfigure {
        fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigure> {
            Box::new(self)
        }
    }

    impl WebSecurityConfigure for TestWebSecurityConfigure {
        fn configure(self) -> HttpSecurity {
            HttpSecurity::default()
                .authorize_http_requests(|mut auth| {
                    auth.request_matchers(vec!["/login", "/logout", "/open"])
                        .permit_all()
                        .request_matchers(HttpMethod::Options)
                        .has_authority("admin")
                        .any_request()
                        .authenticated();
                })
                .form_login(|form| {})
        }
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

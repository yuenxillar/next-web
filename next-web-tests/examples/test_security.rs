use std::{fs::File, io::Read, sync::Arc};

use axum::{response::Html, routing::get};
use next_web::{
    application::Application,
    core::{ApplicationContext, context::properties::ApplicationProperties},
};
use next_web_core::async_trait;
use next_web_security::config::security_builder::SecurityBuilder;
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

        let web_security_configures = ctx.resolve_by_type::<Box<dyn next_web_security::core::web_security_configure::WebSecurityConfigure>>();

        for mut web_security_configure in web_security_configures {
            web_security_configure.configure().build();
        }

        Ok(())
    }
}

mod t2 {
    use std::{any::TypeId, collections::HashMap};

    use next_web::macros::bind::singleton;
    use next_web_core::{
        ApplicationContext, traits::any_clone::AnyClone, util::http_method::HttpMethod,
    };
    use next_web_security::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            web::builders::HttpSecurity,
        },
        core::web_security_configure::WebSecurityConfigure,
        web::util::matcher::Builder,
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
        fn configure(&mut self) -> HttpSecurity {
            let mut ctx = ApplicationContext::default();
            ctx.insert_singleton(Builder::default());

            let mut shared_objects: HashMap<TypeId, Box<dyn AnyClone>> = HashMap::new();
            shared_objects.insert(std::any::TypeId::of::<ApplicationContext>(), Box::new(ctx));

            HttpSecurity::new(AuthenticationManagerBuilder::new(), shared_objects)
                .csrf(|csrf| {
                    csrf.spa();
                })
                .authorize_http_requests(|auth| {
                    auth.request_matchers(vec!["/login", "/logout", "/open"])
                        .permit_all()
                        .request_matchers(HttpMethod::Options)
                        .has_authority("admin")
                        .any_request()
                        .authenticated();
                })
                .security_matchers(|mc| {
                    mc.request_matchers_with_patterns(&["/api/v1/**"])
                        .request_matchers_with_patterns(&["/api/v2/**"]);
                })
        }
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

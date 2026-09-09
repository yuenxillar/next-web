use std::{any::TypeId, borrow::Cow, collections::HashMap, fs::File, io::Read, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::{Html, Response},
    routing::{get, post},
};
use next_web::{
    application::Application,
    core::{ApplicationContext, context::properties::ApplicationProperties},
};
use next_web_context::{ApplicationEvent, ApplicationEventPublisher};
use next_web_core::{
    async_trait,
    error::BoxError,
    filter::{CompositeFilter, application_filter_chain::ApplicationFilterChain},
    traits::{
        any_clone::AnyClone,
        filter::{HttpFilter, HttpFilterChain},
    },
};
use next_web_security::{
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        security_builder::SecurityBuilder,
        web::{WebSecurityConfigurer, builders::HttpSecurity},
    },
    web::{
        filter_chain_proxy::{FilterChainProxy, VirtualFilterChain},
        security_filter_chain::SecurityFilterChain,
        util::matcher::Builder,
    },
};
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
    async fn application_router(&self, ctx: &mut ApplicationContext) -> axum::Router {
        let web_security_configurers = ctx.resolve_by_type::<Box<dyn WebSecurityConfigurer>>();
        let mut ctx = ApplicationContext::default();
        ctx.insert_singleton(Builder::default());
        ctx.insert_singleton::<Arc<dyn ApplicationEventPublisher>>(Arc::new(
            DefaultApplicationEventPublisher,
        ));
        let mut shared_objects: HashMap<TypeId, Box<dyn AnyClone>> = HashMap::new();
        shared_objects.insert(TypeId::of::<ApplicationContext>(), Box::new(ctx));

        #[derive(Clone)]
        struct DefaultApplicationEventPublisher;

        impl ApplicationEventPublisher for DefaultApplicationEventPublisher {
            fn publish_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), BoxError> {
                Ok(())
            }
        }

        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        for mut web_security_configurer in web_security_configurers {
            web_security_configurer.configure(&mut http);
        }
        let chain = http.build();

        let mut filter = CompositeFilter::default();
        filter.set_filters(vec![Arc::new(FilterChainProxy::new(vec![Arc::new(chain)]))]);

        let filter_chain = ApplicationFilterChain::new(vec![Arc::new(filter)]);

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
                    .route("/get", post(async || "Authorized"))
                    .route_layer(axum::middleware::from_fn_with_state(
                        Arc::new(filter_chain) as Arc<dyn HttpFilterChain>,
                        http_filter_layer,
                    )),
            )
    }

    async fn on_ready(
        &self,
        ctx: &mut ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(Vec::<String>::new())), "tokenStore");
        Ok(())
    }
}

type FilterResult = std::result::Result<Response, Response>;

pub async fn http_filter_layer(
    State(filter_chain): State<Arc<dyn HttpFilterChain>>,
    mut req: Request,
    next: Next,
) -> FilterResult {
    let mut resp = Response::new(Body::empty());

    if let Err(err) = filter_chain.do_filter(&mut req, &mut resp).await {
        tracing::error!("The  filter_chain encountered an error: {}", err)
    }

    Ok(next.run(req).await)
}

mod t2 {
    use next_web::macros::bind::singleton;
    use next_web_core::http::HttpMethod;
    use next_web_security::config::web::{WebSecurityConfigurer, builders::HttpSecurity};

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
                csrf.spa();
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
                headers.xss_protection(|xss| {
                    xss.disable();
                });
            })
            .port_mapper(|pm| {
                pm.http(30).maps_to(1000).http(40).maps_to(1010);
            });
        }
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

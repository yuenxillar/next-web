use std::{any::TypeId, collections::HashMap, fs::File, io::Read, sync::Arc};

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
        http::http_response::HttpResponse,
    },
};
use next_web_security::{
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        security_builder::SecurityBuilder,
        web::{WebSecurityConfigurer, builders::HttpSecurity},
    },
    web::{
        filter_chain_proxy::FilterChainProxy, security_filter_chain::SecurityFilterChain,
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
        shared_objects.insert(TypeId::of::<Builder>(), Box::new(Builder::default()));

        #[derive(Clone)]
        struct DefaultApplicationEventPublisher;

        impl ApplicationEventPublisher for DefaultApplicationEventPublisher {
            fn publish_event(&self, _event: Box<dyn ApplicationEvent>) -> Result<(), BoxError> {
                Ok(())
            }
        }

        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects);
        for mut web_security_configurer in web_security_configurers {
            web_security_configurer.configure(&mut http);
        }
        let chain = http.build();

        println!(
            "{}",
            chain
                .get_filters()
                .iter()
                .map(|f| f.name())
                .collect::<Vec<_>>()
                .join(",")
        );
        let filter: Arc<dyn HttpFilter> = {
            let mut filter = CompositeFilter::default();
            filter.set_filters(vec![Arc::new(FilterChainProxy::new(vec![Arc::new(chain)]))]);
            Arc::new(filter)
        };

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
                        filter,
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
    State(filter): State<Arc<dyn HttpFilter>>,
    mut req: Request,
    next: Next,
) -> FilterResult {
    let filter_chain = ApplicationFilterChain::new(vec![filter]);
    let mut resp = Response::new(Body::empty());

    if let Err(err) = filter_chain.do_filter(&mut req, &mut resp).await {
        tracing::error!("{}", err);
        return Err(Response::new(err.to_string().into()));
    }

    if resp.is_committed() {
        return Ok(resp);
    }

    Ok(next.run(req).await)
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
                            r#"{"code": 401, "message": "未认证，请先登录"}"#.as_bytes().to_vec(),
                        );

                        Ok(())
                    },
                ))
                .access_denied_handler(access_denied_handler_fn_wrapper(|_req, resp, _err| {
                    resp.insert_header("Content-Type", "application/json;charset=UTF-8");
                    resp.set_status_code(StatusCode::FORBIDDEN);
                    resp.set_body(r#"{"code": 403, "message": "权限不足"}"#.as_bytes().to_vec());

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
    TestApplication::run().await;
}

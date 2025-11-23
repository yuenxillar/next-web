use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Request, State},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use next_web_core::async_trait;
use next_web_core::state::application_state::ApplicationState;
use next_web_core::traits::filter::http_filter::HttpFilter;
use next_web_dev::context::properties::ApplicationProperties;
use next_web_dev::{
    application::Application, filter::application_filter_chain::ApplicationFilterChain,
    ApplicationContext,
};
use next_web_security::web::filter_proxy::FilterProxy;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().nest(
            "/auth",
            axum::Router::new()
                .route("/hello", get(async || "Hello!"))
                .route("/setToken/{token}", post(set_token))
                .route("/get", post(async || "Authorized"))
                .route_layer(axum::middleware::from_fn_with_state(
                    Arc::new(FilterProxy::default()),
                    security_middleware,
                )),
        )
    }

    async fn on_ready(&self, ctx: &mut ApplicationContext) {
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(Vec::<String>::new())), "tokenStore");
    }
}

async fn security_middleware(
    State(filter_proxy): State<Arc<FilterProxy>>,
    mut req: Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    let mut resp = Response::builder().body(Body::empty()).unwrap();
    let orig_chain = ApplicationFilterChain::default();
    let result = filter_proxy
        .do_filter(&mut req, &mut resp, &orig_chain)
        .await;
    match result {
        Ok(_) => next.run(req).await,
        Err(error) => {
            println!("FilterProxy execute_chain error: {}", error.to_string());
            resp
        }
    }
}

async fn set_token(Path(token): Path<String>, req: Request) -> impl IntoResponse {
    if token.is_empty() {
        return "Error";
    }
    let state = req.extensions().get::<ApplicationState>().unwrap();
    let store = state
        .context()
        .read()
        .await
        .get_single_with_name::<Arc<Mutex<Vec<String>>>>("tokenStore")
        .clone();

    store.lock().await.push(token);
    "Ok"
}

// #[Singleton(binds = [Self::into_authentication_service])]
// #[derive(Clone)]
// struct TestAuthenticationService {
//     #[resource(name = "tokenStore")]
//     store: Arc<Mutex<Vec<String>>>,
// }

// impl TestAuthenticationService {
//     fn into_authentication_service(self) -> Arc<dyn AuthenticationService> {
//         Arc::new(self)
//     }
// }

// #[async_trait]
// impl AuthenticationService for TestAuthenticationService {
//     async fn user_id(&self, req_header: &HeaderMap) -> String {
//         if let Some(auth_header) = req_header.get("Authorization") {
//             let value = auth_header
//                 .to_str()
//                 .unwrap_or_default()
//                 .split(" ")
//                 .last()
//                 .unwrap_or_default()
//                 .to_string();
//             if self.store.lock().await.contains(&value) {
//                 return String::from("admin");
//             }
//         }
//         String::from("user")
//     }

//     async fn login_type(&self, _req_header: &HeaderMap) -> LoginType {
//         LoginType::Username
//     }

//     /// Returns the roles of the user with the given `user_id` and `login_type`.
//     async fn user_role(&self, user_id: &str, _login_type: &LoginType) -> Option<Vec<String>> {
//         if user_id == "admin" {
//             return Some(vec!["admin".into()]);
//         }
//         return None;
//     }

//     /// Returns the permission of the user with the given `user_id` and `login_type`.
//     async fn user_permission(
//         &self,
//         _user_id: &str,
//         _login_type: &LoginType,
//     ) -> Option<Vec<String>> {
//         Some(vec!["*".into()])
//     }
// }

// #[Singleton(binds = [Self::into_web_security_configure])]
// #[derive(Clone)]
// struct TestWebSecurityConfigure;

// impl TestWebSecurityConfigure {
//     fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigure> {
//         Box::new(self)
//     }
// }

// impl WebSecurityConfigure for TestWebSecurityConfigure {
//     fn configure(self) -> HttpSecurity {
//         HttpSecurity::default()
//             .authorize_http_requests(|mut auth| {
//                 auth.request_matchers(vec!["/login", "/logout", "/open"])
//                     .permit_all()
//                     .request_matchers(HttpMethod::Options)
//                     .has_authority("admin")
//                     .any_request()
//                     .authenticated();
//             })
//             .form_login(|form| {})
//     }
// }

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

use std::sync::Arc;

use axum::{
    extract::Path, extract::Request, http::HeaderMap, response::IntoResponse, routing::post,
};
use next_web_core::state::application_state::ApplicationState;
#[allow(missing_docs)]
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{application::Application, Singleton};
use next_web_security::{
    auth::models::login_type::LoginType,
    core::{http_security::HttpSecurity, web_security_configure::WebSecurityConfigure},
    permission::service::authentication_service::AuthenticationService,
};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        ctx.insert_singleton_with_name(Arc::new(Mutex::new(Vec::<String>::new())), "tokenStore");
        axum::Router::new().nest(
            "/login",
            axum::Router::new()
                .route("/setToken/{token}", post(set_token))
                .route("/auth", post(async || "Authorized")),
        )
    }
}
async fn set_token(Path(toekn): Path<String>, req: Request) -> impl IntoResponse {
    if toekn.is_empty() {
        return "Error";
    }
    let state = req.extensions().get::<ApplicationState>().unwrap();
    let store = state
        .get_single_with_name::<Arc<Mutex<Vec<String>>>>("tokenStore")
        .await;

    store.lock().await.push(toekn);
    "Ok"
}

#[Singleton(binds = [Self::into_authentication_service])]
#[derive(Clone)]
struct TestAuthenticationService {
    #[autowired(name = "tokenStore")]
    store: Arc<Mutex<Vec<String>>>,
}

impl TestAuthenticationService {
    fn into_authentication_service(self) -> Arc<dyn AuthenticationService> {
        Arc::new(self)
    }
}

#[async_trait]
impl AuthenticationService for TestAuthenticationService {
    fn user_id(&self, req_header: &HeaderMap) -> String {
        if let Some(auth_header) = req_header.get("Authorization") {
            let value = auth_header
                .to_str()
                .unwrap_or_default()
                .split(" ")
                .last()
                .unwrap_or_default()
                .to_string();
            if self.store.blocking_lock().contains(&value) {
                return "admin".into();
            }
        }
        String::from("user")
    }

    fn login_type(&self, _req_header: &HeaderMap) -> LoginType {
        LoginType::Username
    }

    /// Returns the roles of the user with the given `user_id` and `login_type`.
    async fn user_role(&self, user_id: &str, _login_type: &LoginType) -> Option<Vec<String>> {
        if user_id == "admin" {
            return Some(vec!["admin".into()]);
        }
        return None;
    }

    /// Returns the permission of the user with the given `user_id` and `login_type`.
    async fn user_permission(
        &self,
        _user_id: &str,
        _login_type: &LoginType,
    ) -> Option<Vec<String>> {
        Some(vec!["*".into()])
    }
}

#[Singleton(binds = [Self::into_web_security_configure])]
#[derive(Clone)]
struct TestWebSecurityConfigure;

impl TestWebSecurityConfigure {
    fn into_web_security_configure(self) -> Box<dyn WebSecurityConfigure> {
        Box::new(self)
    }
}

impl WebSecurityConfigure for TestWebSecurityConfigure {
    fn configure(&self) -> next_web_security::core::http_security::HttpSecurity {
        HttpSecurity::new()
            .any_match("/login/auth", |group| group.roles(vec!["admin"]))
            .disable()
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

use std::sync::Arc;

use axum::{
    http::HeaderMap,
    routing::{get, post},
};
#[allow(missing_docs)]
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{application::Application, Singleton};
use next_web_security::{
    auth::models::login_type::LoginType,
    core::{http_security::HttpSecurity, web_security_configure::WebSecurityConfigure},
    permission::service::authentication_service::AuthenticationService,
};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().nest(
            "/login",
            axum::Router::new()
                .route("/test", get(async || "OK"))
                .route("/test2", post(async || "Ok666")),
        )
    }
}

#[Singleton(binds = [Self::into_authentication_service])]
#[derive(Clone)]
struct TestAuthenticationService;

impl TestAuthenticationService {
    fn into_authentication_service(self) -> Arc<dyn AuthenticationService> {
        Arc::new(self)
    }
}

#[async_trait]
impl AuthenticationService for TestAuthenticationService {
    fn user_id(&self, req_header: &HeaderMap) -> String {
        String::from("test_user_id")
    }

    fn login_type(&self, req_header: &HeaderMap) -> LoginType {
        LoginType::Username
    }

    /// Returns the roles of the user with the given `user_id` and `login_type`.
    async fn user_role(&self, user_id: &str, login_type: &LoginType) -> Option<Vec<String>> {
        Some(vec!["user".into()])
    }

    /// Returns the permission of the user with the given `user_id` and `login_type`.
    async fn user_permission(&self, user_id: &str, login_type: &LoginType) -> Option<Vec<String>> {
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
        HttpSecurity::new().any_match("/test3/{*index}", |group| group.roles(vec!["user"]))
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

#![allow(missing_docs)]

use axum::http::StatusCode;
use axum::Router;
use next_web_core::async_trait;
use next_web_core::context::application_resources::{ApplicationResources, ResourceLoader};
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};

use next_web_dev::application::Application;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    async fn before_start(&self, ctx: &mut ApplicationContext) {
        let application_resources = ctx.get_single_with_name::<ApplicationResources>("applicationResources");
        match application_resources.load("/hello.json") {
            Some(data) => {
                println!("{}", String::from_utf8_lossy(data));
            },
            None => {},
        }
    }

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        Router::new().route("/", axum::routing::get(|| async { "Hello, World!" }))
        .route("/test", axum::routing::post(handler))
    }
}

async fn handler() -> impl axum::response::IntoResponse {
    (StatusCode::OK, String::from("{\"message\": \"TestApplication\"}"))
}


#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

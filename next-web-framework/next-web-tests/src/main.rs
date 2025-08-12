#![allow(missing_docs)]

use axum::Router;
use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};

use next_web_dev::application::Application;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        // _ctx.contains_single_with_name
        Router::new().route("/", axum::routing::get(|| async { "Hello, World!" }))
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

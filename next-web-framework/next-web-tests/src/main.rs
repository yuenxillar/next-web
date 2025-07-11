#![allow(missing_docs)]

use axum::Router;
use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};

use next_web_dev::application::Application;

mod test_mqtt;
mod test_redis;
mod test_websocket;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        Router::new()
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

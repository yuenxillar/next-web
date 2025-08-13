#![allow(missing_docs)]

use std::sync::Arc;

use axum::response::IntoResponse;
use axum::Router;
use next_web_core::async_trait;
use next_web_core::interface::singleton::Singleton;
use next_web_core::state::application_state::AcSingleton;
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
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        ctx.insert_singleton_with_name::<Arc<dyn Test>, String>(Arc::new(DDD), "".to_string());
        Router::new()
            .route("/", axum::routing::get(|| async { "Hello, World!" }))
            .route("/login", axum::routing::get(hanlder))
    }
}

#[derive(Clone)]
struct DDD;

impl Test for DDD {
    fn test(&self) -> usize {
        100
    }
}

impl Singleton for DDD {}


trait Test: Send + Sync  {
    fn test(&self) -> usize;
}

async fn hanlder(AcSingleton(test): AcSingleton<Arc<dyn Test>>) -> impl IntoResponse {
    format!("ddd: {}", test.test())
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

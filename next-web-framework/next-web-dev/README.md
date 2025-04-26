# Next Web Dev


Dev - make everything simpler


```rust
use next_web_dev::{ application::Applictaion, router::{open_router::OpenRouter, private_router::PrivateRouter},};
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, ApplicationContext
};
use axum::routing::get;
// use async_trait::async_trait

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;


#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&mut self, properties: &ApplicationProperties) {}

    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> (OpenRouter, PrivateRouter) {
        (OpenRouter::default(), PrivateRouter(axum::Router::new().route("/test", get(test_fn))))
    }
}


async fn test_fn() -> &'static str {
    " Hello Axum! \n Hello Next Web!"
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

```


use async_trait::async_trait;
use next_web_dev::{
    application::application::Application,
    router::{open_router::OpenRouter, private_router::PrivateRouter},
    ApplicationContext, ApplicationProperties,
};


/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&mut self, properties: &ApplicationProperties) {}

    async fn applicatlion_router(
        &self,
        context: &ApplicationContext,
    ) -> (OpenRouter, PrivateRouter) {
        (OpenRouter::default(), PrivateRouter::default())
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

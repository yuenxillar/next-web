use next_web_core::{async_trait, context::properties::{ApplicationProperties, Properties}, ApplicationContext, AutoRegister};
use next_web_dev::{
    application::application::Application, router::{open_router::OpenRouter, private_router::PrivateRouter}, Properties, SingleOwner, Singleton, Transient
};

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;


#[Singleton(name = "testAA")]
#[Properties(prefix = "test")]
#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct TestAA {
    message: Option<String>,
    age: Option<u32>,
}


/// Implementation of `Application` trait for `TestApplication`
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

/// Run the test application
#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

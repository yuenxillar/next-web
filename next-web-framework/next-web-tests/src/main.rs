#![allow(missing_docs)]

use next_web_core::{
    async_trait,
    context::properties::ApplicationProperties,
    ApplicationContext,
};
use next_web_dev::{
    application::application::Application,
    router::{open_router::OpenRouter, private_router::PrivateRouter},
    Properties, Singleton,
};

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[Singleton(binds = [Self::into_properties], default)]
#[Properties(prefix = "test")]
#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct TestBB {
    pub message: Option<String>,
    pub age: Option<u32>,
    #[value = "run1"]
    pub run: Option<bool>
}

pub struct TestModule {
    pub name: String,
    pub age: u32,
    pub message: String,
    pub run: bool,
}

/// Implementation of `Application` trait for `TestApplication`
#[async_trait]
impl Application for TestApplication {
    async fn init_middleware(&mut self, properties: &ApplicationProperties) {}

    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> (OpenRouter, PrivateRouter) {

        let var = ctx.resolve::<TestBB>();
        println!("testbb: {:?}", var);
        (OpenRouter::default(), PrivateRouter::default())
    }
}

/// Run the test application
#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

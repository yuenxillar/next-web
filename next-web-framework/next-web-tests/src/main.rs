#![allow(missing_docs)]

use next_web_core::{
    async_trait,
    context::properties::{ApplicationProperties, Properties},
    ApplicationContext, AutoRegister,
};
use next_web_dev::{
    application::application::Application,
    router::{open_router::OpenRouter, private_router::PrivateRouter},
    Properties, SingleOwner, Singleton,
};

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[Singleton(binds = [Self::into_properties], default)]
#[Properties(prefix = "test")]
#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct TestBB {
    message: Option<String>,
    age: Option<u32>,
    run: Option<bool>
}

#[Singleton(name = "foo")]
#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct TestAA {
    bb: TestBB
}

#[Singleton(name = "number")]
fn Number() -> i32 {
    42
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
        ctx.resolve_with_name::<i32>("number");
        let var = ctx.resolve::<TestBB>();
        println!("testbb: {:?}", var);
        
        let var = ctx.resolve_with_name::<TestAA>("foo");
        println!("testaa: {:?}", var);
       
        (OpenRouter::default(), PrivateRouter::default())
    }
}

/// Run the test application
#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

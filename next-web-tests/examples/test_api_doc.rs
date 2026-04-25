use std::error::Error;

use next_web::{
    application::Application,
    core::{ApplicationContext, async_trait, context::properties::ApplicationProperties},
    extract::find_singleton::FindSingleton,
    macros::bind::{post_mapping, request_mapping},
};
use utoipa::openapi::OpenApi;

#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

// #[api_doc(
//     responses(
//         (status = 200, description = "JSON file", body = ())
//     )
// )]
#[request_mapping(method = "GET", path = "/test")]
async fn openapi(FindSingleton(open_api): FindSingleton<OpenApi>) -> String {
    open_api.to_pretty_json().unwrap()
}

// #[api_doc]
#[post_mapping(path = "/test1")]
async fn test() -> String {
    String::from("Hello world!")
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

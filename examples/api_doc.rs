use next_web::{
    Application, NextWebApplication,
    extract::find_singleton::FindSingleton,
    macros::bind::{post_mapping, request_mapping},
};
use utoipa::openapi::OpenApi;

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

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
    NextWebApplication::<TestApplication>::default().run().await
}

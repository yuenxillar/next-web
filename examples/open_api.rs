use next_web::{
    Application, NextWebApplication,
    macros::{
        bind::{post_mapping, request_mapping},
        open_api,
    },
    open_api::OpenApiProperties,
};
use utoipa::openapi::OpenApi;

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

#[open_api(
    responses(
        (status = 200, description = "JSON file", body = ())
    )
)]
#[request_mapping(method = "GET", path = "/test")]
async fn openapi(FindSingleton(open_api): FindSingleton<OpenApi>) -> String {
    open_api.to_pretty_json().unwrap_or(String::from("Unkown"))
}

// The documentation is described by the `next.open_api` properties of
// `resources/application.yaml`, which are also available as a singleton.
#[open_api(
    responses(
        (status = 200, description = "The information the documentation is served with", body = ())
    )
)]
#[request_mapping(method = "GET", path = "/info")]
async fn info(FindSingleton(properties): FindSingleton<OpenApiProperties>) -> String {
    format!(
        "{} {}: {}",
        properties.title(),
        properties.version(),
        properties.description().unwrap_or_default(),
    )
}

#[open_api]
#[post_mapping(path = "/test1")]
async fn test() -> String {
    String::from("Hello world!")
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

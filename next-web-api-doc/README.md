## Api Doc

A crate for integrating [utoipa](https://crates.io/crates/utoipa) with [next-web](https://crates.io/crates/next-web) Framework for automatic OpenAPI/Swagger documentation generation.

The rules are consistent with utoipa

## WARNING

Due to the limitations of the utoipa, you must add utoipa to your Cargo.toml

```shell
cargo add utoipa
```

## Examples

```rust
use next_web::{
    api_doc, application::Application, async_trait, extract::find_singleton::FindSingleton,
    post_mapping, request_mapping,
};
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};
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
    ) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
}

#[api_doc(
    responses(
        (status = 200, description = "JSON file", body = ())
    )
)]
#[request_mapping(method = "GET", path = "/test")]
async fn openapi(FindSingleton(open_api): FindSingleton<OpenApi>) -> String {
    open_api.to_pretty_json().unwrap()
}

#[api_doc]
#[post_mapping(path = "/test1")]
async fn test() -> String {
    String::from("Hello world!")
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

```

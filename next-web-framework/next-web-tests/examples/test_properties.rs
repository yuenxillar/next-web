use axum::{extract::Path, response::IntoResponse};
use next_web_core::{async_trait, context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{application::Application, middleware::find_singleton::FindSingleton};

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route("/properties", axum::routing::get(req_properties))
        .route("/oneValue/{value}", axum::routing::get(req_one_value))
    }
}

async fn req_properties(
    FindSingleton(properties): FindSingleton<ApplicationProperties>,
) -> impl IntoResponse {
    format!("properties: {:?}", properties)
}


async fn req_one_value(
    FindSingleton(properties): FindSingleton<ApplicationProperties>,
    Path(value): Path<String>,
) -> impl IntoResponse {
    format!("properties: {:?}", properties.one_value::<u32>(&value))
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

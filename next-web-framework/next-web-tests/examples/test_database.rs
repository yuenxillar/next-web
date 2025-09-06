use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::application::Application;

use axum::routing::get;
use next_web_data_database::service::database_service::DatabaseService;
use next_web_dev::middleware::find_singleton::FindSingleton;

/// Test application
#[derive(Default, Clone)] 
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    async fn application_router(&mut self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new().route("/version", get(req_version))
    }
}

async fn req_version(
    FindSingleton(service): FindSingleton<DatabaseService>,
) -> impl axum::response::IntoResponse {
    let version: String = service
        .query_decode("SELECT VERSION();", vec![])
        .await
        .unwrap_or("unknown version".to_string());
    version
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

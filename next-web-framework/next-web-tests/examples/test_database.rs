use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};
use next_web_data_database::transactional::transactionl_executor::TransactionalExecutor;
use next_web_dev::application::Application;

use axum::routing::{get, post};
use next_web_data_database::service::database_service::DatabaseService;
use next_web_dev::middleware::find_singleton::FindSingleton;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    async fn application_router(&self, _ctx: &mut ApplicationContext) -> axum::Router {
        axum::Router::new()
            .route("/version", get(req_version))
            .route("/transaction", post(req_transaction))
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

async fn req_transaction(
    FindSingleton(service): FindSingleton<DatabaseService>,
) -> impl axum::response::IntoResponse {
    match service.execute_transaction(|rbs| async {
        // insert

        // update

        // And Error
        Err("Error".into())
    }).await {
        Ok(_) => {},
        Err(_) => {},
    };
    
    "Ok"
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

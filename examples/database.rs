use next_web::{Application, NextWebApplication};
use next_web_data_database::transactional::transactionl_executor::TransactionalExecutor;

use next_web::extract::find_singleton::FindSingleton;
use next_web::macros::bind::{get_mapping, post_mapping};
use next_web_data_database::service::database_service::DatabaseService;

/// Test application
#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/version")]
async fn req_version(
    FindSingleton(service): FindSingleton<DatabaseService>,
) -> impl axum::response::IntoResponse {
    let version: String = service
        .query_decode("SELECT VERSION();", vec![])
        .await
        .unwrap_or("unknown version".to_string());
    version
}

#[post_mapping(path = "/transaction")]
async fn req_transaction(
    FindSingleton(service): FindSingleton<DatabaseService>,
) -> impl axum::response::IntoResponse {
    match service
        .execute_transaction(|_rbs| async {
            // insert

            // update

            // And Error
            Err("Error".into())
        })
        .await
    {
        Ok(_) => {}
        Err(_) => {}
    };

    "Ok"
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

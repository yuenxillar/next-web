# Next Web Data Database

Database - make everything simpler


# Tips
Currently supports MySQL and PostgreSQL
In theory, What Rbatis supports should be included

# Usage 
If you want to use it, please ensure that the file contains the following content

CARGO_MANIFEST_DIR/resources/application.yaml

# Used in conjunction, otherwise useless

And lib

next-web-dev


```yaml

next:
    data: 
     database:
        driver: mysql
        host: localhost
        port: 3306
        username: root
        password: 123456
        database: test_db

```

```rust
#![allow(missing_docs)]

use next_web_core::async_trait;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};
use next_web_dev::{
    application::Application,
    router::{open_router::OpenRouter, private_router::PrivateRouter}
};


use axum::extract::State;
use axum::routing::get;
use axum::Router;
use next_web_data_database::service::database_service::DatabaseService;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(
        &mut self,
        ctx: &mut ApplicationContext,
    ) -> (OpenRouter, PrivateRouter) {
        let service = ctx
            .get_single_with_name::<DatabaseService>("databaseService")
            .to_owned();
        let interface = Router::new()
            .route("/test_api", get(test_api))
            .with_state(service);
        (OpenRouter::default(), PrivateRouter(interface))
    }
}

async fn test_api(State(service): State<DatabaseService>) -> impl axum::response::IntoResponse {
    let version: String = service
        .query_decode("SELECT VERSION();", vec![])
        .await
        .unwrap();
    version
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

```
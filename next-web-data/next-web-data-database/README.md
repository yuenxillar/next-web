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

next-web


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
use next_web::application::Application;

use axum::routing::get;
use next_web_data_database::service::database_service::DatabaseService;
use next_web::middleware::find_singleton::FindSingleton;

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

```
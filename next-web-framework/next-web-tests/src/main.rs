#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use axum::routing::post;
use axum::Router;
use next_web_core::async_trait;
use next_web_core::core::service::Service;
use next_web_core::core::singleton::Singleton;
use next_web_core::{context::properties::ApplicationProperties, ApplicationContext};

use next_web_dev::application::Application;
use next_web_dev::interceptor::request_data_interceptor::Data;
use next_web_dev::Singleton;
use serde::{Deserialize, Serialize};

mod test_mqtt;
mod test_redis;
mod test_websocket;

/// Test application
#[derive(Default, Clone)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&mut self, ctx: &mut ApplicationContext) -> axum::Router {
        Router::new().route("/test/789", post(test_789))
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct TestData {
    name: String,
    age: i32,
}
async fn test_789(Data(data): Data<TestData>) -> String {
    serde_json::to_string(&data).unwrap()
}

#[derive(Clone)]
pub struct TestService;

impl Singleton  for TestService {}
impl Service    for TestService {}

impl TestColl for TestService {}

pub trait TestColl: Service {
    fn hit(&self) -> String {
        String::from("hit")
    }
}

#[Singleton]
#[derive(Clone)]
pub struct TestContext {
    #[autowired(vec)]
    pub services: Vec<TestService>,
    #[autowired(map)]
    pub value: HashMap<String, Arc<dyn TestColl>>,
}

#[tokio::main]
async fn main() {
    let service = TestService;
    let service1 = TestService;
    let _ref_service: Arc<dyn Service> = Arc::new(service);
    let _ref_service1: Arc<dyn Service> = Arc::new(service1);
    TestApplication::run().await;
}


#[cfg(test)]
mod test_project {
    use next_web_core::core::service::{Service, TestBBH};

    use crate::TestService;


    #[test]
    fn test_all() {
        let service = TestService;

        let path = service.path();
        println!("{:?}", path);
    }

    impl TestBBH for TestService {
        
    }
}
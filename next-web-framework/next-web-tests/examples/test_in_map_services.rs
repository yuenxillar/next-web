use std::{collections::HashMap, fmt::Debug, sync::Arc};

use axum::response::IntoResponse;
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, interface::service::Service,
    state::application_state::AcSingleton, ApplicationContext,
};
use next_web_dev::{application::Application, Singleton};

#[Singleton(binds = [Self::into_test_coll])]
#[derive(Clone, Debug)]
pub struct TestService;

pub trait TestColl: Service {
    fn coll(&self) -> String {
        String::from("coll")
    }
}

#[Singleton(binds = [Self::into_test_coll])]
#[derive(Clone)]
pub struct TestService1;

impl TestService1 {
    fn into_test_coll(self) -> Arc<dyn TestColl> {
        Arc::new(self)
    }
}

impl TestService {
    fn into_test_coll(self) -> Arc<dyn TestColl> {
        Arc::new(self)
    }
}

impl Service for TestService {}
impl Service for TestService1 {}

impl TestColl for TestService1 {}
impl TestColl for TestService {}

#[Singleton]
#[derive(Clone)]
pub struct TestMapService {
    #[autowired(map)]
    pub services_map: HashMap<String, Arc<dyn TestColl>>,
}

#[derive(Clone, Default)]
struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&mut self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(
        &mut self,
        _ctx: &mut ApplicationContext,
    ) -> axum::Router {
        axum::Router::new().route("/getMapService", axum::routing::get(get_service))
    }
}
async fn get_service(map_service: AcSingleton<TestMapService>) -> impl IntoResponse {
    map_service
        .services_map
        .keys()
        .map(|s| s.to_owned())
        .collect::<Vec<String>>()
        .join(":")
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

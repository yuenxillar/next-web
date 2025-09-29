use std::sync::Arc;

use axum::{response::IntoResponse, Router};
use next_web_core::{
    async_trait, context::properties::ApplicationProperties, traits::singleton::Singleton,
    ApplicationContext,
};
use next_web_dev::{
    application::Application, middleware::find_singleton::FindSingleton, Singleton,
};

#[derive(Clone, Default)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    /// initialize the middleware.
    async fn init_middleware(&self, _properties: &ApplicationProperties) {}

    // get the application router. (open api  and private api)
    async fn application_router(&self, _ctx: &mut ApplicationContext) -> Router {
        Router::new().route("/getSingleton", axum::routing::get(req_get_singleton))
    }
}

async fn req_get_singleton(
    FindSingleton(test): FindSingleton<TestSingletonConsumer>,
) -> impl IntoResponse {
    format!(
        "single: {}\nsingle_vec: [{}]\nsingle_map: [{}]\nsingle_option: {}",
        test.single.get_name(),
        test.single_vec
            .iter()
            .map(|s| s.get_name())
            .collect::<Vec<_>>()
            .join(","),
        test.single_map
            .iter()
            .map(|(k, _v)| k.clone())
            .collect::<Vec<_>>()
            .join(","),
        test.single_option
            .map(|s| s.get_name())
            .unwrap_or("unfound".to_string())
    )
}

/// 我将定义一个 动态 trait
/// I will define a dynamic trait

pub trait TestSingleton: Send + Sync
where
    Self: Singleton,
{
    fn get_name(&self) -> String {
        self.singleton_name()
    }
}

/// 多个实现者
/// Multiple implementers
#[Singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplOne;

#[Singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplTwo;

#[Singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplThree;

impl TestSingleton for TestSingletonImplOne {}
impl TestSingleton for TestSingletonImplTwo {}
impl TestSingleton for TestSingletonImplThree {}


impl TestSingletonImplOne {
    fn into_test_singleton(self: Self) -> Arc<dyn TestSingleton> {
        Arc::new(self)
    }
}

impl TestSingletonImplTwo {
    fn into_test_singleton(self: Self) -> Arc<dyn TestSingleton> {
        Arc::new(self)
    }
}

impl TestSingletonImplThree {
    fn into_test_singleton(self: Self) -> Arc<dyn TestSingleton> {
        Arc::new(self)
    }
}
/// 获取单例
/// Get singleton
#[Singleton]
#[derive(Clone)]
pub struct TestSingletonConsumer {
    #[resource(name = "testSingletonImplOne")]
    pub single: Arc<dyn TestSingleton>,

    #[resource(vec)]
    pub single_vec: Vec<Arc<dyn TestSingleton>>,

    // 这里 V 的泛型需要实现 Singleton trait 然后单例名称为 K
    #[resource(map)]
    pub single_map: std::collections::HashMap<String, Arc<dyn TestSingleton>>,

    #[resource(name = "testSingletonImplThree", option)]
    pub single_option: Option<Arc<dyn TestSingleton>>,
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

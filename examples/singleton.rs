use std::sync::Arc;

use next_web::{
    Application, NextWebApplication,
    extract::find_singleton::FindSingleton,
    macros::bind::{get_mapping, singleton},
};
use next_web_core::traits::singleton::Singleton;

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/getSingleton")]
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
#[singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplOne;

#[singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplTwo;

#[singleton(binds = [Self::into_test_singleton])]
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
#[singleton]
#[derive(Clone)]
pub struct TestSingletonConsumer {
    #[autowired(name = "testSingletonImplOne")]
    pub single: Arc<dyn TestSingleton>,

    #[autowired(vec)]
    pub single_vec: Vec<Arc<dyn TestSingleton>>,

    // 这里 V 的泛型需要实现 Singleton trait 然后单例名称为 K
    #[autowired(map)]
    pub single_map: std::collections::HashMap<String, Arc<dyn TestSingleton>>,

    #[autowired(name = "testSingletonImplThree", option)]
    pub single_option: Option<Arc<dyn TestSingleton>>,
}

#[singleton]
fn test_name(a: &String, b: &String, c: &String) -> String {
    format!("{}{}{}", a, b, c)
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

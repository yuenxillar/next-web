use std::sync::Arc;

use next_web::{
    Application, NextWebApplication,
    macros::bind::{get_mapping, singleton},
};

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

#[get_mapping(path = "/getSingleton")]
async fn req_get_singleton(
    FindSingleton(test): FindSingleton<TestSingletonConsumer>,
) -> impl IntoResponse {
    format!(
        "single_vec: {}\nsingle_map: [{}]\nsingle_option: {}\napplication_name: {}",
        !test.single_vec.is_empty(),
        test.single_map
            .iter()
            .map(|(k, _v)| k.clone())
            .collect::<Vec<_>>()
            .join(","),
        test.single_option.is_some(),
        test.application_name
    )
}

/// A dynamic trait for test singletons.
///
/// Any type implementing this trait must also implement `Singleton`,
/// which provides `singleton_name()` for identifying the singleton.
pub trait TestSingleton: Send + Sync {}

// Multiple implementers of `TestSingleton`.

/// First implementer.
///
/// `binds = [Self::into_test_singleton]` registers this type as a
/// `TestSingleton` via the conversion function `into_test_singleton`.
#[singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplOne;

/// Second implementer.
#[singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplTwo;

/// Third implementer.
#[singleton(binds = [Self::into_test_singleton])]
#[derive(Clone)]
struct TestSingletonImplThree;

impl TestSingleton for TestSingletonImplOne {}
impl TestSingleton for TestSingletonImplTwo {}
impl TestSingleton for TestSingletonImplThree {}

impl TestSingletonImplOne {
    /// Converts `self` into a trait object `Arc<dyn TestSingleton>`.
    fn into_test_singleton(self: Self) -> Arc<dyn TestSingleton> {
        Arc::new(self)
    }
}

impl TestSingletonImplTwo {
    /// Converts `self` into a trait object `Arc<dyn TestSingleton>`.
    fn into_test_singleton(self: Self) -> Arc<dyn TestSingleton> {
        Arc::new(self)
    }
}

impl TestSingletonImplThree {
    /// Converts `self` into a trait object `Arc<dyn TestSingleton>`.
    fn into_test_singleton(self: Self) -> Arc<dyn TestSingleton> {
        Arc::new(self)
    }
}

/// A consumer that autowires several `TestSingleton` instances.
///
/// Demonstrates four autowiring styles:
/// - `name = "..."`: inject a specific named singleton
/// - `vec`: collect all registered singletons into a `Vec`
/// - `map`: collect all registered singletons into a `HashMap`
///   keyed by singleton name
/// - `option`: inject a specific singleton, or `None` if not found
#[singleton]
#[derive(Clone)]
pub struct TestSingletonConsumer {
    /// Inject the singleton named `testSingletonImplOne`.
    #[autowired(name = "testSingletonImplOne")]
    pub single: Arc<dyn TestSingleton>,

    /// Inject every registered `TestSingleton` as a `Vec`.
    #[autowired(vec)]
    pub single_vec: Vec<Arc<dyn TestSingleton>>,

    /// Inject every registered `TestSingleton` as a `HashMap`.
    #[autowired(map)]
    pub single_map: std::collections::HashMap<String, Arc<dyn TestSingleton>>,

    /// Inject the singleton named `testSingletonImplThree` if present.
    #[autowired(name = "testSingletonImplThree", option)]
    pub single_option: Option<Arc<dyn TestSingleton>>,

    /// The name of the application. Retrieve from configuration file
    ///
    /// Read from the configuration key `next.application.name`.
    /// If the key is missing, defaults to `"TestApplication"`.
    #[value(key = "next.application.name", default = String::from("TestApplication"))]
    application_name: String,
}

/// A singleton function that concatenates three strings.
#[singleton]
fn test_name(a: &String, b: &String, c: &String) -> String {
    format!("{}{}{}", a, b, c)
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

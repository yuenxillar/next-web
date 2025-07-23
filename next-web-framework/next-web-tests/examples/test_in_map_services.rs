use std::{any::Any, collections::HashMap, fmt::Debug, sync::Arc};

use next_web_core::{
    core::{service::Service, singleton::Singleton},
    utils::singleton_util::find_group_singleton,
    ApplicationContext,
};
use next_web_dev::Singleton;

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
pub struct TestContext {
    #[autowired(map)]
    pub services_map: HashMap<String, Arc<dyn TestColl>>,
}

fn main() {
    let mut ctx = ApplicationContext::auto_register();

    let colls: Vec<Arc<dyn TestColl>> = ctx.resolve_by_type::<Arc<dyn TestColl>>();

    // colls.into_iter().filter(|i| find_group_singleton(singleton));

    let cal = TestService;
    let box_cal: Box<dyn TestColl> = Box::new(cal);
    let ref_cal: &dyn Any = &box_cal;
    println!("{:#?}", ref_cal.downcast_ref::<Box<dyn Service>>()
            .map(|s| s.group()))
}

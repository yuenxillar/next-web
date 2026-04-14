use std::sync::Arc;

use next_web::{
    ApplicationContext, AutoRegister,
    application::Application,
    async_trait,
    context::properties::ApplicationProperties,
    macros::{autoconfigure::auto_configuration, bind::singleton},
};
use next_web_core::error::BoxError;

#[derive(Clone, Default)]
pub struct TestApplication;

#[async_trait]
impl Application for TestApplication {
    type ErrorSolve = ();

    /// initialize the middleware.
    async fn init_middleware(
        &self,
        _ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[derive(Clone)]
#[singleton(binds = [Self::into_auto])]
pub struct TestAutoRegister;

#[async_trait]
impl AutoRegister for TestAutoRegister {
    fn name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), BoxError> {
        ctx.insert_singleton_with_name(String::from("value1"), "msg2");
        ctx.insert_singleton_with_name(String::from("value0"), "s3");

        ctx.insert_singleton_with_name(Arc::new(String::from("value0")), "value");

        Ok(())
    }
}

impl TestAutoRegister {
    pub fn into_auto(self) -> Arc<dyn AutoRegister> {
        Arc::new(self)
    }
}

#[derive(Clone)]
pub struct TestAutoConfiguation;

#[auto_configuration]
impl TestAutoConfiguation {
    #[provider(name = "msg666", conditional = [Self::test1], order = 12)]
    fn msg1(
        #[autowired(name = "msg2")] s1: String,
        #[autowired(default)] s2: String,
        s3: String,
    ) -> String {
        format!("{}:{}:{}", s1, s2, s3)
    }

    #[conditional_on_property(name = "next.feature.enabled", having_value = "true")]
    #[provider]
    fn msg2() -> Vec<String> {
        println!("is me!");
        vec![]
    }

    #[provider]
    async fn msg3(value: Arc<String>, #[autowired(name = "msg2")] msg2: String) -> Vec<String> {
        let msg1 = value.as_ref().clone();

        vec![msg1, msg2]
    }

    fn test1(ctx: &ApplicationContext) -> bool {
        ctx.contains_single_with_name::<String>("s2")
    }
}

#[tokio::main]
async fn main() {
    TestApplication::run().await;
}

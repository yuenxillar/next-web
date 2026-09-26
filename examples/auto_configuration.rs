use std::sync::Arc;

use next_web::{
    Application, NextWebApplication,
    core::{
        ApplicationContext, async_trait, traits::config::auto_configuration::AutoConfiguration,
    },
    macros::{
        autoconfigure::{auto_configuration, configuration_properties},
        bind::singleton,
    },
};
use next_web_context::ApplicationContextExt;
use next_web_core::Ordered;

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

/// The properties of the feature the condition of the auto-configuration reads.
#[configuration_properties(prefix = "next.feature")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestFeatureProperties {
    pub enabled: Option<bool>,
}

#[derive(Clone)]
#[singleton(binds = [Self::into_auto_configuration])]
pub struct TestAutoRegister;

#[async_trait]
impl AutoConfiguration for TestAutoRegister {
    async fn configure(
        &mut self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ctx.insert_singleton_with_name(String::from("value1"), "msg2");
        ctx.insert_singleton_with_name(String::from("value0"), "s3");

        ctx.insert_singleton_with_name(Arc::new(String::from("value0")), "value");

        Ok(())
    }
}

impl Ordered for TestAutoRegister {
    fn order(&self) -> i32 {
        100
    }
}

impl TestAutoRegister {
    pub fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
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

    #[provider(conditional = [Self::feature_enabled])]
    fn msg2() -> Vec<String> {
        println!("is me!");
        vec![]
    }

    #[provider]
    async fn msg3(value: Arc<String>, #[autowired(name = "msg2")] msg2: String) -> Vec<String> {
        let msg1 = value.as_ref().clone();

        vec![msg1, msg2]
    }

    fn test1(ctx: &dyn ApplicationContext) -> bool {
        ctx.contains_singleton_with_name::<String>("s2")
    }

    /// Returns whether the feature of the application is enabled.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the properties of the feature are read from.
    fn feature_enabled(ctx: &dyn ApplicationContext) -> bool {
        ctx.get_singleton_option_with_default_name::<TestFeatureProperties>()
            .and_then(|properties| properties.enabled)
            .unwrap_or(false)
    }
}

#[tokio::main]
async fn main() {
    NextWebApplication::<TestApplication>::default().run().await
}

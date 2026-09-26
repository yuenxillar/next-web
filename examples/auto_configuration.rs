//! The auto-configuration example.
//!
//! The example declares the auto-configuration of an application in both of the
//! ways the framework supports:
//!
//! - a type implements `AutoConfiguration` and is provided as a singleton, which
//!   is how a starter contributes its configuration,
//! - an `impl` block is annotated with `#[auto_configuration]`, which turns the
//!   methods that declare a provider into the singletons of the application.
//!
//! Both are applied in the order of `Ordered`, and a configuration is skipped
//! when its condition does not hold. The example serves the singletons it
//! created at `http://127.0.0.1:11000/autoConfiguration`.

use std::collections::HashMap;
use std::sync::Arc;

use next_web::{
    core::{
        async_trait, traits::config::auto_configuration::AutoConfiguration, ApplicationContext,
    },
    extract::find_singleton::FindSingleton,
    macros::{
        autoconfigure::{auto_configuration, configuration_properties},
        bind::{get_mapping, singleton},
    },
    Application, NextWebApplication,
};
use next_web_context::ApplicationContextExt;
use next_web_core::{anys::any_value::AnyValue, Ordered};

#[derive(Default)]
pub struct TestApplication;

impl Application for TestApplication {}

/// The properties the condition of the configuration class reads.
///
/// The example enables the feature while it starts, so the conditional provider
/// of the configuration class is created.
#[configuration_properties(prefix = "next.feature")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TestFeatureProperties {
    pub enabled: Option<bool>,
}

/// The instance the configuration class of the example creates.
#[derive(Debug, Clone)]
pub struct Greeting {
    /// The message the greeting was built with.
    pub message: String,

    /// The names of the feature, which the conditional provider contributed.
    pub names: Vec<String>,
}

/// An auto-configuration the application provides itself.
///
/// A starter contributes its auto-configurations the same way: with a singleton
/// that is bound to `Box<dyn AutoConfiguration>`.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct TestAutoRegister;

impl TestAutoRegister {
    pub fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for TestAutoRegister {
    async fn configure(
        &mut self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // What the configuration class below depends on.
        ctx.insert_singleton_with_name(String::from("value1"), "applicationName");
        ctx.insert_singleton_with_name(Arc::new(String::from("value0")), "value");

        Ok(())
    }
}

impl Ordered for TestAutoRegister {
    fn order(&self) -> i32 {
        100
    }
}

/// The type the configuration class below belongs to.
///
/// The type only gives the configuration a name of its own; the methods that
/// declare a provider are called by the framework while the application starts.
#[derive(Clone)]
pub struct TestAutoConfiguration;

/// The configuration class of the application.
///
/// The class is applied after the auto-configuration above, which is what lets
/// its providers depend on the singletons that auto-configuration created.
#[auto_configuration(order = 200)]
impl TestAutoConfiguration {
    /// A conditional provider, which runs before the other providers of the
    /// class so that they can depend on the names it contributes.
    #[provider(name = "featureNames", conditional = [Self::feature_enabled], order = 10)]
    fn feature_names() -> Vec<String> {
        println!("the feature of the example is enabled");

        vec![String::from("first"), String::from("second")]
    }

    /// A provider that names its instance and resolves its parameters by name.
    ///
    /// The names of the feature are optional, so the instance is created whether
    /// the feature of the example is enabled or not.
    #[provider]
    fn greeting(
        #[autowired(name = "applicationName")] application_name: String,
        #[autowired(name = "featureNames", default)] names: Vec<String>,
        value: Arc<String>,
    ) -> Greeting {
        Greeting {
            message: format!("{application_name}: {value}"),
            names,
        }
    }

    /// An asynchronous provider.
    #[provider]
    async fn repeated(value: Arc<String>) -> Vec<String> {
        vec![value.as_ref().clone(), value.as_ref().clone()]
    }

    /// Returns whether the feature of the example is enabled.
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

/// Serves the instance the configuration class created.
#[get_mapping(path = "/autoConfiguration")]
async fn auto_configuration(FindSingleton(greeting): FindSingleton<Greeting>) -> impl IntoResponse {
    format!("{greeting:?}")
}

#[tokio::main]
async fn main() {
    let mut application = NextWebApplication::<TestApplication>::default();

    // The condition of the configuration class reads this property, and a
    // default property makes the example behave the same way wherever it runs.
    application.set_default_properties(HashMap::from([(
        String::from("next.feature.enabled"),
        AnyValue::from(true),
    )]));

    application.run().await
}

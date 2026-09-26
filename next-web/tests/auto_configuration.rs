//! The auto-configuration of an application.
//!
//! The test drives the auto-configurations through [`WebAutoConfiguration`],
//! which is what the configuration channel is about: a configuration class
//! declared with `#[auto_configuration]` is applied the same way as the
//! auto-configuration of a starter, in the order of `Ordered`, and only when its
//! conditions hold.

use std::sync::Mutex;

use next_web::{
    autoconfigure::web_auto_configuration::WebAutoConfiguration,
    context::{ApplicationContextExt, DefaultApplicationContext},
    core::{
        async_trait, traits::config::auto_configuration::AutoConfiguration, ApplicationContext,
    },
    macros::{autoconfigure::auto_configuration, bind::singleton},
    ConfigurableApplicationContext,
};
use next_web_core::Ordered;

/// The configurations of the test that were applied, in the order they were.
static APPLIED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

/// Serializes the tests, which share the configurations of this binary.
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Records that a configuration was applied.
///
/// # Arguments
///
/// * `name` - The name of the configuration, or of its provider.
fn record(name: &'static str) {
    APPLIED.lock().unwrap().push(name);
}

/// An auto-configuration the test provides itself, which runs before the
/// configuration class below and creates the instance it depends on.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
struct ProvidedAutoConfiguration;

impl ProvidedAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for ProvidedAutoConfiguration {
    async fn configure(
        &mut self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        record("auto-configuration");
        ctx.insert_singleton_with_name(String::from("early"), "earlyName");

        Ok(())
    }
}

impl Ordered for ProvidedAutoConfiguration {
    fn order(&self) -> i32 {
        10
    }
}

/// The configuration class of the test.
#[derive(Clone)]
struct TestConfiguration;

#[auto_configuration(order = 20, conditional = [Self::enabled])]
impl TestConfiguration {
    /// A provider that runs before the providers of the default order, which is
    /// what lets them depend on the name it registers.
    #[provider(name = "earlyNames", order = 10)]
    fn early_names(#[autowired(name = "earlyName")] early: String) -> Vec<String> {
        record("first provider");

        vec![early]
    }

    /// A provider that resolves its parameters by name, including one that is
    /// not registered and falls back to its default.
    #[provider(name = "lateName")]
    fn late(
        #[autowired(name = "earlyNames")] names: Vec<String>,
        #[autowired(name = "missing", default)] missing: String,
    ) -> String {
        record("second provider");

        format!("late:{}:{missing}", names.join(","))
    }

    /// A provider whose property condition holds.
    #[provider(name = "conditionalName")]
    #[conditional_on_property(name = "AUTO_CONFIGURATION_PROPERTY", having_value = "true")]
    fn conditional() -> String {
        record("conditional provider");

        String::from("conditional")
    }

    /// A provider whose property condition does not hold, so it is skipped.
    #[provider(name = "skippedPropertyName")]
    #[conditional_on_property(name = "AUTO_CONFIGURATION_PROPERTY", having_value = "false")]
    fn skipped_property() -> String {
        record("provider with a false property condition");

        String::from("skipped")
    }

    /// Returns whether the configuration is applied.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the condition is read from.
    fn enabled(ctx: &dyn ApplicationContext) -> bool {
        ctx.contains_singleton_with_name::<String>("featureEnabled")
    }
}

/// A configuration class whose condition does not hold, so it is not applied.
#[derive(Clone)]
struct DisabledConfiguration;

#[auto_configuration(order = 30, conditional = [Self::enabled])]
impl DisabledConfiguration {
    #[provider(name = "disabledName")]
    fn disabled() -> String {
        record("configuration with a false condition");

        String::from("disabled")
    }

    /// Returns whether the configuration is applied.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the condition is read from.
    fn enabled(ctx: &dyn ApplicationContext) -> bool {
        ctx.contains_singleton_with_name::<String>("featureDisabled")
    }
}

/// Starts a context and applies the auto-configurations of the application.
async fn start_application() -> DefaultApplicationContext {
    let mut context = DefaultApplicationContext::default();
    context.refresh().expect("the context refreshes");

    // The condition of the configuration class of the test reads this
    // singleton, which the test registers before the configurations run.
    context.insert_singleton_with_name(String::from("enabled"), "featureEnabled");

    let mut auto_configuration = WebAutoConfiguration;
    auto_configuration
        .configure(&mut context)
        .await
        .expect("the application is configured");

    context
}

#[tokio::test]
async fn the_configurations_of_the_application_are_applied_in_their_order() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    std::env::set_var("AUTO_CONFIGURATION_PROPERTY", "true");
    APPLIED.lock().unwrap().clear();

    let context = start_application().await;

    // The auto-configuration of the test ran before the configuration class,
    // which resolved the instance it created.
    assert_eq!(
        context
            .get_singleton_option_with_name::<String>("lateName")
            .map(String::as_str),
        Some("late:early:"),
        "the configuration class resolves the instances of the auto-configurations"
    );

    // The provider of the held condition ran, the one of the property that does
    // not hold was skipped, and so was the configuration that is not enabled.
    assert_eq!(
        context
            .get_singleton_option_with_name::<String>("conditionalName")
            .map(String::as_str),
        Some("conditional")
    );
    assert_eq!(
        context.get_singleton_option_with_name::<String>("skippedPropertyName"),
        None
    );
    assert_eq!(
        context.get_singleton_option_with_name::<String>("disabledName"),
        None
    );

    assert_eq!(
        *APPLIED.lock().unwrap(),
        vec![
            "auto-configuration",
            "first provider",
            "second provider",
            "conditional provider",
        ]
    );
}

#[tokio::test]
async fn a_configuration_class_is_an_auto_configuration_of_the_application() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    std::env::set_var("AUTO_CONFIGURATION_PROPERTY", "true");
    APPLIED.lock().unwrap().clear();

    let mut context = start_application().await;

    // Every configuration of the test, the ones of the application and the ones
    // of the configuration classes, is resolvable as an auto-configuration,
    // together with the ones the framework contributes itself.
    assert!(
        context
            .resolve_by_type::<Box<dyn AutoConfiguration>>()
            .len()
            >= 3,
        "the configuration classes of the test are auto-configurations of the application"
    );
}

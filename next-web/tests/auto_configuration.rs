//! The auto-configuration of an application.
//!
//! The test drives the auto-configurations through [`WebAutoConfiguration`],
//! which is what the configuration channel is about: a configuration class
//! declared with `#[auto_configuration]` is applied the same way as the
//! auto-configuration of a starter, in the order of `Ordered` refined by the
//! configurations it declares it has to be applied before and after, and only
//! when its conditions hold.

use std::sync::{Arc, Mutex};

use next_web::{
    ConfigurableApplicationContext,
    autoconfigure::web_auto_configuration::WebAutoConfiguration,
    context::{ApplicationContextExt, DefaultApplicationContext},
    core::{
        ApplicationContext, async_trait,
        env::{ConfigurableEnvironment, MapPropertySource, StandardEnvironment},
        traits::config::auto_configuration::AutoConfiguration,
    },
    macros::{autoconfigure::auto_configuration, bind::singleton},
};
use next_web_core::{Ordered, util::indexmap::IndexMap};

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

/// A configuration class that declares the place it has to hold relative to
/// another auto-configuration, which wins over the order it declares.
#[derive(Clone)]
struct ConstrainedConfiguration;

#[auto_configuration(order = 200, before = ["ProvidedAutoConfiguration"])]
impl ConstrainedConfiguration {
    #[provider(name = "constrainedName")]
    fn constrained() -> String {
        record("configuration with a before constraint");

        String::from("constrained")
    }
}

/// The instance the providers of the configuration below share.
#[derive(Clone, Debug)]
struct Counter {
    value: u32,
}

/// A configuration class whose providers borrow the instances of the context.
///
/// A reference parameter borrows the instance the context holds instead of
/// resolving a copy of it, which is what lets a provider change the instance the
/// rest of the application sees.
#[derive(Clone)]
struct BorrowConfiguration;

#[auto_configuration(order = 40)]
impl BorrowConfiguration {
    /// Creates the instance the other providers of the configuration borrow.
    #[provider(name = "counter")]
    fn counter() -> Counter {
        Counter { value: 0 }
    }

    /// Two references to the instance of the same name are the same instance.
    #[provider(name = "sameInstance")]
    fn same_instance(
        #[autowired(name = "counter")] first: &Counter,
        #[autowired(name = "counter")] second: &Counter,
    ) -> bool {
        std::ptr::eq(first, second)
    }

    /// A mutable reference changes the instance the context holds.
    #[provider(name = "incremented")]
    fn increment(#[autowired(name = "counter")] counter: &mut Counter) -> u32 {
        counter.value += 1;

        counter.value
    }
}

/// Starts a context and applies the auto-configurations of the application.
async fn start_application() -> DefaultApplicationContext {
    // The condition of the configuration class reads
    // `AUTO_CONFIGURATION_PROPERTY` from the environment. The property is
    // installed directly, so the test does not have to change the environment
    // of the process.
    let mut environment = StandardEnvironment::new();
    environment
        .property_sources()
        .add_first(Box::new(MapPropertySource::new(
            "test".to_owned(),
            IndexMap::from([("AUTO_CONFIGURATION_PROPERTY".to_owned(), "true".to_owned())]),
        )));

    let mut context = DefaultApplicationContext::default();
    context.set_environment(Arc::new(environment));
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

    APPLIED.lock().unwrap().clear();

    let context = start_application().await;

    // The configuration that declares it is applied before
    // `ProvidedAutoConfiguration` ran first, even though it declares the higher
    // order, and registered the instance of its provider.
    assert_eq!(
        context
            .get_singleton_option_with_name::<String>("constrainedName")
            .map(String::as_str),
        Some("constrained")
    );

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
            "configuration with a before constraint",
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

#[tokio::test]
async fn the_parameters_of_a_provider_borrow_the_instances_of_the_context() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    APPLIED.lock().unwrap().clear();

    let context = start_application().await;

    // Two references of the same name are the same instance, which the provider
    // of the test reads from the addresses of its parameters: a reference
    // borrows the instance of the context instead of a copy of it.
    assert_eq!(
        context.get_singleton_option_with_name::<bool>("sameInstance"),
        Some(&true)
    );

    // The mutable reference changed the instance the context holds, so the value
    // the application reads afterwards is the changed one.
    assert_eq!(
        context.get_singleton_option_with_name::<u32>("incremented"),
        Some(&1)
    );
    assert_eq!(
        context
            .get_singleton_option_with_name::<Counter>("counter")
            .map(|counter| counter.value),
        Some(1)
    );
}

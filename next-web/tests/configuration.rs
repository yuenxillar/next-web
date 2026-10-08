//! Integration test of the `#[configuration]` attribute.
//!
//! The test declares a configuration with two providers and an initializer, and
//! applies it to a context, which is what a real application does while it
//! starts.

use next_web::context::DefaultApplicationContext;
use next_web::macros::autoconfigure::configuration;
use next_web_context::ApplicationContextExt;
use next_web_core::traits::config::auto_configuration_sorter::AutoConfigurationSorter;
use next_web_core::traits::config::configuration::Configuration;

/// The registry the initializer of the configuration fills.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    /// The names the initializer registered.
    pub names: Vec<String>,
}

/// The type the configuration is declared for.
struct TestConfiguration;

/// A configuration with two providers and an initializer that uses them.
#[configuration]
impl TestConfiguration {
    #[provider(name = "registry")]
    fn registry() -> Registry {
        Registry::default()
    }

    #[provider(name = "greeting")]
    fn greeting() -> String {
        String::from("hello")
    }

    #[init]
    fn register(registry: &mut Registry, greeting: String) {
        registry.names.push(greeting);
    }
}

#[tokio::test]
async fn applies_a_configuration_and_runs_its_initializer() {
    let mut context = DefaultApplicationContext::default();
    context.register_auto_providers();

    let configurations = context.resolve_by_type::<Box<dyn Configuration>>();
    let mut configurations =
        AutoConfigurationSorter::sort(configurations).expect("the configurations are orderable");

    for configuration in &mut configurations {
        configuration
            .configure(&mut context)
            .await
            .expect("the configuration is applied");
    }

    let registry = context.get_singleton_with_name::<Registry>("registry");

    assert_eq!(registry.names, vec![String::from("hello")]);
}

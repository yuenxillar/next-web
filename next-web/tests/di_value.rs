//! `#[value(key = "…", default = …)]` reads a property out of the application's
//! environment.
//!
//! The instance is *configured* rather than injected: nothing resolves a
//! singleton for the field, the environment is asked for the property. That is
//! why this test sets the environment of the context before refreshing it — the
//! context registers its environment while it is prepared, and the provider reads
//! it when it is built.
//!
//! A property that is not configured is not an error: the field takes the value
//! the attribute names, or `Default::default()` when it names none. An `Option`
//! field is how a property that may be absent is asked for.

use std::sync::Arc;

use next_web::{
    ConfigurableApplicationContext,
    context::{ApplicationContextExt, DefaultApplicationContext},
    core::env::{ConfigurableEnvironment, MapPropertySource, StandardEnvironment},
    macros::bind::singleton,
};

/// The values the properties are read into.
#[singleton]
#[derive(Clone, Debug)]
pub struct ServerInfo {
    /// A property the environment holds.
    #[value(key = "test.value.port")]
    port: u16,

    /// A property it does not hold, with the value the attribute names.
    #[value(key = "test.value.absent", default = String::from("stand-in"))]
    named: String,

    /// A property it does not hold, with no value named: the type's own default.
    #[value(key = "test.value.also_absent")]
    count: u32,

    /// A property it holds, read into an `Option` because it may not be there.
    #[value(key = "test.value.name")]
    name: Option<String>,

    /// A property it does not hold, which an `Option` allows.
    #[value(key = "test.value.not_there")]
    missing: Option<String>,
}

/// Starts a context whose environment holds some of the properties above.
fn start_context() -> DefaultApplicationContext {
    let mut environment = StandardEnvironment::new();
    environment
        .property_sources()
        .add_first(Box::new(MapPropertySource::new(
            "test".to_owned(),
            [
                ("test.value.port".to_owned(), "8080".to_owned()),
                ("test.value.name".to_owned(), "next".to_owned()),
            ]
            .into_iter()
            .collect(),
        )));

    let mut context = DefaultApplicationContext::default();
    context.set_environment(Arc::new(environment) as Arc<dyn ConfigurableEnvironment>);
    context.insert_singleton_with_name(false, "mainAllowOverride");
    context.refresh().expect("the context refreshes");

    context
}

#[test]
fn reads_a_property_and_defaults_the_ones_that_are_not_configured() {
    let mut context = start_context();

    let info = context.resolve_with_name::<ServerInfo>("serverInfo");

    assert_eq!(
        info.port, 8080,
        "a configured property is parsed into its type"
    );
    assert_eq!(
        info.named, "stand-in",
        "a property that is not configured takes the value the attribute names"
    );
    assert_eq!(
        info.count, 0,
        "with no value named, an absent property takes the type's own default"
    );
    assert_eq!(
        info.name.as_deref(),
        Some("next"),
        "an optional property that is configured is wrapped in `Some`"
    );
    assert_eq!(
        info.missing, None,
        "an optional property that is not configured is `None`"
    );
}

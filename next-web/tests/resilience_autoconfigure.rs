//! The resilience instances an application's properties describe.
//!
//! This is the only test that drives the whole path a configuration takes: the
//! properties are bound, the auto-configuration is discovered and built by the
//! dependency-injection machinery, and what it registers is checked. Everything
//! else about the resilience configuration is covered closer to the code, where a
//! failure names the field that was wrong.
//!
//! What it is here to catch is a coupling no unit test can see: the `#[singleton]`
//! on the auto-configuration resolves each of its fields by a name derived from
//! the *field*, and that name has to be the one the properties macro gave the
//! struct. Nothing checks that until an application is built.
//!
//! It is one test rather than several because the registry it asserts on is
//! process-wide, so two tests in this binary would see each other's instances.

use std::sync::Arc;

use next_web::{
    ConfigurableApplicationContext,
    autoconfigure::web_auto_configuration::WebAutoConfiguration,
    context::{ApplicationContextExt, DefaultApplicationContext},
    core::{
        config::AutoConfiguration,
        env::{ConfigurableEnvironment, MapPropertySource, StandardEnvironment},
    },
};
use next_web_core::util::indexmap::IndexMap;
use next_web_resilience::{ratelimiter::RateLimiter, registry::global_registry};

/// The instance this test configures. The name is distinctive because the
/// registry outlives the application that filled it.
const INSTANCE: &str = "configured_backend";

/// Starts a context that reads the given properties, the way the framework does
/// at startup, and applies the auto-configurations of the application.
///
/// `configure` applies them, so everything they register is in place by the time
/// this returns.
async fn start(properties: &[(&str, &str)]) -> DefaultApplicationContext {
    let mut environment = StandardEnvironment::new();
    environment
        .property_sources()
        .add_first(Box::new(MapPropertySource::new(
            "test".to_owned(),
            properties
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect::<IndexMap<_, _>>(),
        )));

    let mut context = DefaultApplicationContext::default();
    context.set_environment(Arc::new(environment));

    // `NextWebApplication` registers this while preparing the context, before the
    // context is refreshed, and the refresh reads it back. Driving the
    // auto-configuration directly means doing it here, and in this order.
    context.insert_singleton_with_name(false, "mainAllowOverride");

    context.refresh().expect("the context refreshes");

    let mut auto_configuration = WebAutoConfiguration;
    auto_configuration
        .configure(&mut context)
        .await
        .expect("the application is configured");

    context
}

#[tokio::test]
async fn the_instances_a_properties_file_describes_are_registered_and_configured() {
    // An application whose properties describe no instance is the ordinary case.
    // The auto-configuration still runs, and has to register nothing.
    let _ = start(&[("next.application.name", "no_resilience_configured")]).await;
    assert!(
        global_registry().rate_limiter_names().is_empty(),
        "an application that configures no instance registered some anyway: {:?}",
        global_registry().rate_limiter_names()
    );

    // Now one that does.
    let context = start(&[
        (
            "next.ratelimiter.instances.configured_backend.limitForPeriod",
            "5",
        ),
        (
            "next.ratelimiter.instances.configured_backend.limitRefreshPeriod",
            "1s",
        ),
        (
            "next.ratelimiter.instances.configured_backend.timeoutDuration",
            "0",
        ),
    ])
    .await;

    let limiter = global_registry()
        .find_rate_limiter(INSTANCE)
        .unwrap_or_else(|| {
            panic!(
                "the configured limiter never reached the registry. Either the auto-configuration \
             was not discovered, or the name it resolves its properties under does not match the \
             one the properties struct was given. Registered: {:?}",
                global_registry().rate_limiter_names()
            )
        });

    // What the properties said, not the defaults.
    assert_eq!(
        limiter.config().limit_for_period(),
        5,
        "the settings should come from the properties rather than the defaults"
    );
    assert_eq!(
        limiter.config().limit_refresh_period(),
        std::time::Duration::from_secs(1)
    );
    assert_eq!(
        limiter.config().timeout_duration(),
        std::time::Duration::ZERO
    );

    // The context holds the very same instance, so the two routes to it — the
    // registry the attribute macros look in, and the container — cannot drift
    // apart.
    let from_context = context
        .get_singleton_option_with_name::<Arc<RateLimiter>>(INSTANCE)
        .expect("the configured limiter is in the context too");
    assert!(Arc::ptr_eq(&limiter, from_context));

    // The application declared instances, so the registry stops defaulting names
    // it does not know. A misspelling is the likely mistake now that names are
    // used exactly as written, and this turns it into a report rather than a
    // limiter quietly running with settings nobody chose.
    assert!(
        global_registry().is_strict_names(),
        "declaring instances should turn on the report for undeclared names"
    );

    // An undeclared name is reported and then runs on the defaults: the mistake is
    // loud in the log, and the endpoint stays up rather than panicking per request.
    let undeclared = global_registry().rate_limiter_or_default("configured_backend_typo");
    assert_eq!(
        undeclared.config().limit_for_period(),
        50,
        "the call is served on the defaults rather than refused"
    );
}

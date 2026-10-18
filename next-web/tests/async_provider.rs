//! The resolution of a provider whose constructor is an `async fn`.
//!
//! A context resolves its providers synchronously, which cannot await such a
//! constructor, so the provider is resolved with the asynchronous accessors of
//! [`ApplicationContextAsyncExt`].

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use next_web::{
    ConfigurableApplicationContext,
    context::{ApplicationContextAsyncExt, ApplicationContextExt, DefaultApplicationContext},
    macros::bind::singleton,
};

/// Serializes the tests, which share the providers and the counters of this
/// binary.
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// The number of times the constructor of the asynchronous provider ran.
static CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

/// The number of times the constructor of the eager provider ran.
static EAGERLY_CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

/// The instance the asynchronous provider creates.
#[derive(Clone, Debug)]
pub struct AsyncService {
    /// The name the instance was created with.
    pub name: String,
}

/// A provider whose constructor is an `async fn`.
///
/// The constructor awaits, which is what a synchronous resolution of the
/// context cannot do.
#[singleton]
async fn async_service(#[autowired(name = "baseName")] base: String) -> AsyncService {
    CONSTRUCTED.fetch_add(1, Ordering::SeqCst);

    tokio::task::yield_now().await;

    AsyncService {
        name: format!("{base}:async"),
    }
}

/// The instance the provider that has to exist eagerly creates.
#[derive(Clone, Debug)]
pub struct EagerService {
    /// The name the instance was created with.
    pub name: String,
}

/// A provider whose instance has to exist before the application starts, and
/// whose constructor is an `async fn`.
#[singleton(eager_create)]
async fn eager_service(#[autowired(name = "baseName")] base: String) -> EagerService {
    EAGERLY_CONSTRUCTED.fetch_add(1, Ordering::SeqCst);

    tokio::task::yield_now().await;

    EagerService {
        name: format!("{base}:eager"),
    }
}

/// A value the fields of the struct below collect every instance of.
#[derive(Clone, Debug)]
pub struct NamedValue(pub String);

/// The provider of the value the collected fields resolve.
#[singleton]
fn named_value() -> NamedValue {
    NamedValue(String::from("named"))
}

/// A provider whose fields the dependency injection macro resolves, which
/// awaits them because the constructor of the struct is asynchronous.
#[singleton(async = true)]
#[derive(Clone, Debug)]
pub struct AsyncFields {
    /// The value the context hands out by name.
    #[autowired(name = "baseName")]
    base: String,

    /// The value that falls back to its default, because the name is unknown.
    #[autowired(name = "missingName", default)]
    fallback: String,

    /// The value that may be absent.
    #[autowired(name = "missingName", option)]
    optional: Option<String>,

    /// Every instance of the collected type.
    #[autowired(vec)]
    collected: Vec<NamedValue>,

    /// Every instance of the collected type, keyed by its name.
    #[autowired(map)]
    collected_by_name: HashMap<String, NamedValue>,
}

/// Starts the context of the test, which holds the instance the provider of the
/// test resolves.
fn start_context() -> DefaultApplicationContext {
    let mut context = DefaultApplicationContext::default();
    context.refresh().expect("the context refreshes");
    context.insert_singleton_with_name(String::from("base"), "baseName");

    context
}

/// Starts the context of the test with the asynchronous refresh, which awaits
/// the providers that are created asynchronously.
fn start_context_async_refreshed() -> DefaultApplicationContext {
    let mut context = DefaultApplicationContext::default();
    context.insert_singleton_with_name(String::from("base"), "baseName");

    context
}

#[tokio::test]
async fn resolves_a_provider_whose_constructor_is_asynchronous() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    CONSTRUCTED.store(0, Ordering::SeqCst);

    let mut context = start_context();

    let service = context
        .resolve_with_name_async::<AsyncService>("asyncService")
        .await;

    assert_eq!(service.name, "base:async");
    assert_eq!(CONSTRUCTED.load(Ordering::SeqCst), 1);

    // The instance is a singleton of the context, so resolving it again hands
    // out the instance that was created once.
    let service = context
        .resolve_with_name_async::<AsyncService>("asyncService")
        .await;

    assert_eq!(service.name, "base:async");
    assert_eq!(CONSTRUCTED.load(Ordering::SeqCst), 1);
}

#[test]
#[should_panic(expected = "is created asynchronously")]
fn does_not_resolve_a_provider_whose_constructor_is_asynchronous_synchronously() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut context = start_context();

    let _ = context.resolve_with_name::<AsyncService>("asyncService");
}

#[tokio::test]
async fn creates_the_instances_of_the_asynchronous_providers_eagerly() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    EAGERLY_CONSTRUCTED.store(0, Ordering::SeqCst);

    let mut context = start_context_async_refreshed();
    context
        .refresh_async()
        .await
        .expect("the context refreshes");

    // The instance of the provider that asks for an eager instance exists once
    // the context is refreshed, without a resolution of it.
    assert_eq!(EAGERLY_CONSTRUCTED.load(Ordering::SeqCst), 1);
    assert_eq!(
        context
            .get_singleton_option_with_name::<EagerService>("eagerService")
            .map(|service| service.name.as_str()),
        Some("base:eager")
    );
}

#[tokio::test]
async fn creates_the_instances_of_the_asynchronous_providers_when_they_are_resolved() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    EAGERLY_CONSTRUCTED.store(0, Ordering::SeqCst);

    // A synchronous refresh cannot await the constructor of the provider, so
    // the instance is created when it is resolved instead.
    let mut context = start_context();

    assert_eq!(EAGERLY_CONSTRUCTED.load(Ordering::SeqCst), 0);

    let service = context
        .resolve_with_name_async::<EagerService>("eagerService")
        .await;

    assert_eq!(service.name, "base:eager");
    assert_eq!(EAGERLY_CONSTRUCTED.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn resolves_the_fields_of_an_asynchronous_constructor() {
    let _test = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut context = start_context();

    let fields = context
        .resolve_with_name_async::<AsyncFields>("asyncFields")
        .await;

    assert_eq!(fields.base, "base");
    assert_eq!(fields.fallback, "");
    assert_eq!(fields.optional, None);
    assert_eq!(fields.collected.len(), 1);
    assert_eq!(fields.collected[0].0, "named");
    assert_eq!(
        fields
            .collected_by_name
            .get("namedValue")
            .map(|value| value.0.as_str()),
        Some("named")
    );
}

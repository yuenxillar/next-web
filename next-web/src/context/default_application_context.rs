//! Default [`ConfigurableApplicationContext`] implementation.
//!
//! [`DefaultApplicationContext`] is the context created by
//! `DefaultApplicationContextFactory` when an application does not install a
//! factory of its own. It assembles the pieces an application needs while it
//! starts up:
//!
//! - a [`ConfigurableEnvironment`] holding the properties and the profiles,
//! - a [`DefaultListableSingletonFactory`] acting as the bean container,
//! - an [`ApplicationStartup`] used to record startup steps,
//! - the registered [`ApplicationListener`]s,
//! - an optional [`MessageSource`] used to resolve localized messages.
//!
//! # Lifecycle
//!
//! A context starts out inactive. [`ConfigurableApplicationContext::refresh`]
//! activates it and [`ConfigurableApplicationContext::close`] shuts it down for
//! good. Both flags live in a small shared state (`LifecycleState`), which the
//! shutdown hook can update without borrowing the context mutably.
//!
//! # Listeners
//!
//! [`ConfigurableApplicationContext::add_application_listener`] only receives a
//! trait object, so a listener cannot be identified by its type name. Listeners
//! are therefore keyed by the [`TypeId`] of their concrete type: registering a
//! second listener of the same type replaces the first one, and
//! [`DefaultApplicationContext::listener_id_of`] returns the identifier required
//! to remove a listener again.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use next_web_context::{
    ApplicationContext, ApplicationEvent, ApplicationEventPublisher, ApplicationListener,
    DynProvider, InstanceClone, Locale, MessageSource, MessageSourceResolvable,
    NoSuchMessageError, Scope,
};
use next_web_core::{
    env::ConfigurableEnvironment,
    error::BoxError,
    metrics::{ApplicationStartup, DefaultApplicationStartup},
};
use next_web_singletons::factory::{
    support::{DefaultListableSingletonFactory, DynSingle, Key},
    SingletonFactory,
};

use crate::{
    configurable_application_context::ContextError, ApplicationEnvironment,
    ApplicationShutdownHook, ConfigurableApplicationContext,
};

/// Prefix used for the identifiers handed out by [`DefaultApplicationContext::default`].
const APPLICATION_ID_PREFIX: &str = "application";

/// Source of the sequence numbers used for the default context identifiers.
static APPLICATION_ID_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Lifecycle flags shared between the context and its shutdown hook.
///
/// The context owns the singletons and therefore cannot let the shutdown hook
/// borrow it, but the hook still has to be able to report that the context is no
/// longer usable. These two flags are the only state the hook needs.
#[derive(Debug, Default)]
struct LifecycleState {
    /// Set once `refresh` has completed successfully.
    active: AtomicBool,
    /// Set once `close` (or the shutdown hook) has run.
    closed: AtomicBool,
}

/// The default [`ConfigurableApplicationContext`] implementation.
///
/// See the [module documentation](self) for the pieces the context assembles
/// and for how listeners are identified.
pub struct DefaultApplicationContext {
    /// Unique identifier of this context instance.
    id: String,
    /// Name of the deployed application this context belongs to.
    application_name: String,
    /// Wall clock time (milliseconds since the Unix epoch) of the last refresh.
    startup_date: i64,
    /// Optional parent context, normally an ancestor of this context.
    parent: Option<Arc<dyn ApplicationContext>>,
    /// The environment holding the properties and the profiles.
    environment: Arc<dyn ConfigurableEnvironment>,
    /// Collector recording the steps of the startup phase.
    application_startup: Box<dyn ApplicationStartup>,
    /// The bean container held by this context.
    singleton_factory: DefaultListableSingletonFactory,
    /// Registered listeners, in registration order, paired with their identifier.
    listeners: Vec<(
        String,
        Box<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    )>,
    /// Message source used to resolve localized messages, when one is installed.
    message_source: Option<Arc<dyn MessageSource>>,
    /// Lifecycle flags shared with the shutdown hook.
    lifecycle: Arc<LifecycleState>,
    /// Shutdown hook registered for this context, created on demand.
    shutdown_hook: Option<ApplicationShutdownHook>,
    /// Providers registered with this context, keyed by the instance they create.
    providers: HashMap<Key, DynProvider>,
    /// Keys of the providers that are being created, used to detect cycles.
    resolving: Vec<Key>,
    /// Whether a provider may replace another provider with the same key.
    allow_override: bool,
}

impl DefaultApplicationContext {
    /// Creates an empty application context.
    ///
    /// The environment and the startup collector are set to their defaults, and
    /// a unique identifier is generated.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty application context with the given identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier to install on the context.
    pub fn with_id(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Self::default()
        }
    }

    /// Sets the name of the deployed application this context belongs to.
    ///
    /// The name is returned by [`ApplicationContext::application_name`] and is
    /// empty until it is set here.
    ///
    /// # Arguments
    ///
    /// * `application_name` - The application name.
    pub fn set_application_name(&mut self, application_name: impl Into<String>) {
        self.application_name = application_name.into();
    }

    /// Sets the message source used to resolve localized messages.
    ///
    /// Messages are looked up in this source first, then in the parent context,
    /// before the default message is returned.
    ///
    /// # Arguments
    ///
    /// * `message_source` - The message source to install.
    pub fn set_message_source(&mut self, message_source: Arc<dyn MessageSource>) {
        self.message_source = Some(message_source);
    }

    /// Returns the message source of this context, if one has been installed.
    ///
    /// # Returns
    ///
    /// The installed message source, or `None`.
    pub fn message_source(&self) -> Option<&dyn MessageSource> {
        self.message_source.as_deref()
    }

    /// Returns the shutdown hook registered for this context, if any.
    ///
    /// The returned handle shares its callback registry and cancellation token
    /// with the context, so triggering the shutdown through it marks the context
    /// as closed.
    ///
    /// # Returns
    ///
    /// A clone of the registered hook, or `None` while no hook is registered.
    pub fn shutdown_hook(&self) -> Option<ApplicationShutdownHook> {
        self.shutdown_hook.clone()
    }

    /// Sets whether a provider may replace another provider with the same key.
    ///
    /// # Arguments
    ///
    /// * `allow_override` - Whether replacing a provider is allowed.
    pub fn set_allow_override(&mut self, allow_override: bool) {
        self.allow_override = allow_override;
    }

    /// Registers a provider, together with the providers it is bound to.
    ///
    /// # Panics
    ///
    /// Panics when the key of the provider is already registered and
    /// [`Self::set_allow_override`] was called with `false`.
    ///
    /// # Arguments
    ///
    /// * `provider` - The provider to register.
    pub fn register_provider(&mut self, mut provider: DynProvider) {
        // A provider that binds a type registers one provider per bound type.
        if let Some(bindings) = provider.binding_providers() {
            for binding in bindings {
                self.register_provider(binding);
            }
        }

        let key = provider.key().clone();
        let definition = provider.definition().clone();

        if self.providers.contains_key(&key) && !self.allow_override {
            panic!("a provider with the same key is already registered: {definition:?}");
        }

        self.providers.insert(key, provider);
    }

    /// Registers every provider that was submitted with `register_provider!`.
    ///
    /// This is what makes the providers of the attribute macros available to the
    /// context without the application having to list them.
    pub fn register_auto_providers(&mut self) {
        for provider in next_web_context::auto_registered_providers() {
            self.register_provider(provider);
        }
    }

    /// Returns the identifier that
    /// [`ConfigurableApplicationContext::add_application_listener`] assigns to a
    /// listener of type `L`.
    ///
    /// The trait method receives a trait object, whose type name no longer
    /// identifies the concrete listener, so the identifier is derived from the
    /// concrete type instead. Use this helper to build the identifier accepted
    /// by [`ConfigurableApplicationContext::remove_application_listener`]:
    ///
    /// ```ignore
    /// context.add_application_listener(Box::new(listener));
    /// context.remove_application_listener(
    ///     &DefaultApplicationContext::listener_id_of::<MyListener>(),
    /// );
    /// ```
    pub fn listener_id_of<L>() -> String
    where
        L: ApplicationListener<Box<dyn ApplicationEvent>> + 'static,
    {
        Self::listener_id(TypeId::of::<L>())
    }

    /// Formats the [`TypeId`] of a concrete listener type as its identifier.
    fn listener_id(type_id: TypeId) -> String {
        format!("{type_id:?}")
    }

    /// Returns `Ok(())` while the context is active.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::IllegalState`] when the context has not been
    /// refreshed yet or has already been closed.
    fn check_active(&self, operation: &str) -> Result<(), ContextError> {
        if self.is_active() {
            Ok(())
        } else {
            Err(ContextError::IllegalState(format!(
                "cannot {operation} an application context that is not active"
            )))
        }
    }

    /// Resolves `code` against the message source of this context and then
    /// against the parent context.
    ///
    /// # Errors
    ///
    /// Returns [`NoSuchMessageError`] when neither the installed message source
    /// nor the parent context knows the code.
    fn resolve_message(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        if let Some(message_source) = self.message_source.as_deref() {
            if let Ok(message) = message_source.message(code, args, locale) {
                return Ok(message);
            }
        }

        if let Some(parent) = self.parent.as_deref() {
            if let Ok(message) = parent.message(code, args, locale) {
                return Ok(message);
            }
        }

        Err(NoSuchMessageError::new(code, locale))
    }
}

impl Default for DefaultApplicationContext {
    fn default() -> Self {
        Self {
            id: next_application_id(),
            application_name: String::new(),
            startup_date: current_time_millis(),
            parent: None,
            environment: Arc::new(ApplicationEnvironment::default()),
            application_startup: Box::new(DefaultApplicationStartup::default()),
            singleton_factory: DefaultListableSingletonFactory::default(),
            listeners: Vec::new(),
            message_source: None,
            lifecycle: Arc::new(LifecycleState::default()),
            shutdown_hook: None,
            providers: HashMap::new(),
            resolving: Vec::new(),
            allow_override: true,
        }
    }
}

impl ConfigurableApplicationContext for DefaultApplicationContext {
    fn set_id(&mut self, id: String) {
        self.id = id;
    }

    fn set_parent(&mut self, parent: Option<Arc<dyn ApplicationContext>>) {
        self.parent = parent;
    }

    fn set_environment(&mut self, environment: Arc<dyn ConfigurableEnvironment>) {
        self.environment = environment;
    }

    fn environment(&self) -> &dyn ConfigurableEnvironment {
        self.environment.as_ref()
    }

    fn set_application_startup(&mut self, application_startup: Box<dyn ApplicationStartup>) {
        self.application_startup = application_startup;
    }

    fn application_startup(&self) -> &dyn ApplicationStartup {
        self.application_startup.as_ref()
    }

    fn add_application_listener(
        &mut self,
        listener: Box<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    ) {
        let id = Self::listener_id(listener.as_ref().type_id());

        // A concrete listener type is registered at most once. Adding another
        // instance of the same type replaces the previous one, which mirrors the
        // set semantics of the original Spring implementation.
        match self.listeners.iter_mut().find(|(key, _)| *key == id) {
            Some((_, registered)) => *registered = listener,
            None => self.listeners.push((id, listener)),
        }
    }

    fn remove_application_listener(&mut self, id: &str) {
        self.listeners.retain(|(listener_id, _)| listener_id != id);
    }

    fn refresh(&mut self) -> Result<(), ContextError> {
        if self.is_closed() {
            return Err(ContextError::IllegalState(
                "cannot refresh an application context that has been closed".to_owned(),
            ));
        }

        if self.is_active() {
            return Err(ContextError::IllegalState(
                "the application context has already been refreshed".to_owned(),
            ));
        }

        // Singletons that have to exist eagerly are created while refreshing,
        // mirroring the "finishRefresh" phase of the original Spring context.
        self.singleton_factory.initialize_defaults();
        self.startup_date = current_time_millis();
        self.lifecycle.active.store(true, Ordering::SeqCst);

        Ok(())
    }

    fn restart(&mut self) -> Result<(), ContextError> {
        // Nothing in this context tracks the lifecycle of individual singletons
        // yet, so restarting only verifies that it is legal to do so.
        self.check_active("restart")
    }

    fn pause(&mut self) -> Result<(), ContextError> {
        // See `restart`: pausing has no per singleton effect yet.
        self.check_active("pause")
    }

    fn register_shutdown_hook(&mut self) {
        if self.shutdown_hook.is_some() {
            // A context registers at most one shutdown hook.
            return;
        }

        let lifecycle = Arc::clone(&self.lifecycle);
        let shutdown_hook = ApplicationShutdownHook::new();
        shutdown_hook.add_hook(move || async move {
            // The hook cannot borrow the context, so it only marks it as closed;
            // `close` remains responsible for releasing the singletons.
            lifecycle.active.store(false, Ordering::SeqCst);
            lifecycle.closed.store(true, Ordering::SeqCst);
        });

        self.shutdown_hook = Some(shutdown_hook);
    }

    fn close(&mut self) -> Result<(), ContextError> {
        if self.is_closed() {
            // Closing is idempotent.
            return Ok(());
        }

        // Release the singletons held by this context. The parent context is
        // deliberately left untouched, because it has its own lifecycle.
        self.singleton_factory.clear_all_singletons();
        self.listeners.clear();
        self.lifecycle.active.store(false, Ordering::SeqCst);
        self.lifecycle.closed.store(true, Ordering::SeqCst);

        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.lifecycle.closed.load(Ordering::SeqCst)
    }

    fn is_active(&self) -> bool {
        !self.is_closed() && self.lifecycle.active.load(Ordering::SeqCst)
    }

    fn parent(&self) -> Option<&dyn ApplicationContext> {
        self.parent.as_deref()
    }

    fn singleton_factory(&mut self) -> Result<&mut DefaultListableSingletonFactory, ContextError> {
        if self.is_closed() {
            return Err(ContextError::IllegalState(
                "the application context has been closed".to_owned(),
            ));
        }

        // The factory is available from construction until the context is
        // closed: the application has to populate it before refreshing.
        Ok(&mut self.singleton_factory)
    }
}

impl ApplicationContext for DefaultApplicationContext {
    fn application_name(&self) -> &str {
        &self.application_name
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn startup_date(&self) -> i64 {
        self.startup_date
    }

    fn insert_singleton_boxed(
        &mut self,
        key: Key,
        instance: Box<dyn Any + Send + Sync>,
        clone: Option<InstanceClone>,
    ) {
        // The context delegates the storage of its singletons to the singleton
        // factory, which keeps the registry in a single place.
        self.singleton_factory
            .registry_mut()
            .insert_dyn(key, DynSingle::new(instance, clone));
    }

    fn contains_singleton(&self, key: &Key) -> bool {
        self.singleton_factory.registry().contains_key(key)
    }

    fn get_singleton_boxed(&self, key: &Key) -> Option<&(dyn Any + Send + Sync)> {
        self.singleton_factory
            .registry()
            .get_dyn(key)
            .map(DynSingle::as_any)
    }

    fn get_singleton_boxed_mut(&mut self, key: &Key) -> Option<&mut (dyn Any + Send + Sync)> {
        self.singleton_factory
            .registry_mut()
            .get_dyn_mut(key)
            .map(DynSingle::as_any_mut)
    }

    fn remove_singleton_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        self.singleton_factory
            .registry_mut()
            .remove_dyn(key)
            .map(DynSingle::into_inner_boxed)
    }

    fn resolve_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        // An instance that was created before is handed out as it is.
        if let Some(instance) = self.singleton_factory.registry().get_dyn(key) {
            return instance.get_owned_boxed();
        }

        let provider = self.providers.get(key)?.clone();
        let definition = provider.definition().clone();
        let scope = definition.scope;

        if scope == Scope::SingleOwner {
            // A single owner instance belongs to the context, so it can only be
            // borrowed from it once it was created.
            panic!("a single owner instance cannot be resolved as an owned value: {definition:?}");
        }

        if self.resolving.contains(key) {
            panic!("a circular dependency was detected while resolving: {definition:?}");
        }

        self.resolving.push(key.clone());
        let instance = provider.build_boxed(self);
        self.resolving.pop();

        if scope != Scope::Singleton {
            // A transient instance is created for every resolution and is not
            // stored in the context.
            return Some(instance);
        }

        let erased_clone: Option<InstanceClone> = provider.clone_instance().map(|clone| {
            let origin = provider.origin();

            let erased: InstanceClone =
                Arc::new(move |instance: &(dyn Any + Send + Sync)| {
                    clone(origin.as_ref(), instance)
                });

            erased
        });

        match erased_clone {
            Some(clone) => {
                // Keep one copy in the context and return the other one.
                let stored = clone(instance.as_ref());
                self.singleton_factory
                    .registry_mut()
                    .insert_dyn(key.clone(), DynSingle::new(stored, Some(Arc::clone(&clone))));

                Some(instance)
            }
            None => {
                // The instance cannot be copied, so it can only be borrowed from
                // the context afterwards.
                self.singleton_factory
                    .registry_mut()
                    .insert_dyn(key.clone(), DynSingle::new(instance, None));

                None
            }
        }
    }

    fn resolve_all_of_type(&mut self, ty: TypeId) -> Vec<Box<dyn Any + Send + Sync>> {
        // An instance either comes from a provider, or was inserted into the
        // context directly.
        let mut keys: Vec<Key> = self
            .providers
            .keys()
            .filter(|key| key.ty.id == ty)
            .cloned()
            .collect();
        keys.extend(self.singleton_factory.registry().keys_of_type(ty));
        keys.sort();
        keys.dedup();

        keys.iter()
            .filter_map(|key| self.resolve_boxed(key))
            .collect()
    }
}

impl ApplicationEventPublisher for DefaultApplicationContext {
    fn publish_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), BoxError> {
        // Listeners are notified in registration order. Each of them receives
        // its own clone of the event, so a listener cannot consume the event for
        // the listeners that follow.
        for (_, listener) in &self.listeners {
            block_on(listener.on_application_event(event.clone()));
        }

        // The event is also handed to the parent context, which notifies its own
        // listeners, matching the behaviour of the original implementation.
        if let Some(parent) = self.parent.as_deref() {
            parent.publish_event(event)?;
        }

        Ok(())
    }
}

impl MessageSource for DefaultApplicationContext {
    fn message(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        self.resolve_message(code, args, locale)
    }

    fn message_from_resolvable(
        &self,
        resolvable: &dyn MessageSourceResolvable,
        locale: Option<&Locale>,
    ) -> Result<String, NoSuchMessageError> {
        let codes = resolvable.codes().unwrap_or_default();
        let arguments = resolvable.arguments().map(|arguments| {
            arguments
                .iter()
                .map(|argument| argument.as_ref())
                .collect::<Vec<&dyn fmt::Display>>()
        });

        for code in codes {
            if let Ok(message) = self.resolve_message(code, arguments.as_deref(), locale) {
                return Ok(message);
            }
        }

        // The default message of the resolvable is the last resort, exactly like
        // the default message of `message_or_default`.
        if let Some(default_message) = resolvable.default_message() {
            return Ok(default_message.to_owned());
        }

        Err(NoSuchMessageError::new(
            codes.last().map(String::as_str).unwrap_or_default(),
            locale,
        ))
    }

    fn message_or_default(
        &self,
        code: &str,
        args: Option<&[&dyn fmt::Display]>,
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String> {
        self.resolve_message(code, args, locale)
            .ok()
            .or_else(|| default_message.map(|message| message.to_owned()))
    }
}

/// Returns the next unique identifier for an application context.
fn next_application_id() -> String {
    let sequence = APPLICATION_ID_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    format!("{APPLICATION_ID_PREFIX}-{sequence}")
}

/// Returns the current wall clock time in milliseconds since the Unix epoch.
fn current_time_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}

/// Drives a listener future to completion for the synchronous
/// [`ApplicationEventPublisher`] contract.
///
/// Inside a multi threaded Tokio runtime the current worker is kept available
/// with [`tokio::task::block_in_place`], so listeners that need the runtime keep
/// working. Everywhere else the future is driven by the `futures` executor.
fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(|| handle.block_on(future))
        }
        _ => futures::executor::block_on(future),
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::pin::Pin;
    use std::sync::atomic::AtomicUsize;

    use next_web_singletons::factory::config::SingletonRegistry;
    use next_web_context::{
        ApplicationContextExt, Provider, singleton, transient,
    };

    use super::*;

    /// Minimal application event used to observe event publishing.
    #[derive(Clone)]
    struct TestEvent;

    impl ApplicationEvent for TestEvent {
        fn source(&self) -> &dyn Any {
            self
        }

        fn event_type(&self) -> TypeId {
            TypeId::of::<Self>()
        }

        fn source_type(&self) -> TypeId {
            TypeId::of::<Self>()
        }
    }

    /// Listener counting the events it receives.
    struct CountingListener {
        received: Arc<AtomicUsize>,
    }

    impl ApplicationListener<Box<dyn ApplicationEvent>> for CountingListener {
        fn on_application_event<'a>(
            &'a self,
            _event: Box<dyn ApplicationEvent>,
        ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            self.received.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {})
        }
    }

    #[test]
    fn default_context_is_usable_but_inactive() {
        let context = DefaultApplicationContext::default();

        assert!(!context.id().is_empty());
        assert_eq!(context.application_name(), "");
        assert!(context.startup_date() > 0);
        assert!(!context.is_active());
        assert!(!context.is_closed());
        assert!(context.parent().is_none());
    }

    #[test]
    fn distinct_contexts_get_distinct_identifiers() {
        let first = DefaultApplicationContext::default();
        let second = DefaultApplicationContext::default();

        assert_ne!(first.id(), second.id());

        let renamed = DefaultApplicationContext::with_id("custom");
        assert_eq!(renamed.id(), "custom");
    }

    #[test]
    fn refresh_activates_the_context_only_once() {
        let mut context = DefaultApplicationContext::default();

        assert!(context.restart().is_err());
        assert!(context.pause().is_err());

        context.refresh().expect("the first refresh succeeds");
        assert!(context.is_active());
        assert!(context.restart().is_ok());
        assert!(context.pause().is_ok());

        assert!(context.refresh().is_err());
    }

    #[test]
    fn close_is_idempotent_and_releases_the_singletons() {
        let mut context = DefaultApplicationContext::default();
        context.refresh().expect("refreshing succeeds");
        context
            .singleton_factory()
            .expect("the factory is open")
            .registry_mut()
            .register_singleton("config", String::from("value"));

        context.close().expect("closing succeeds");

        assert!(context.is_closed());
        assert!(!context.is_active());
        assert!(context.close().is_ok());
        assert!(context.singleton_factory().is_err());
    }

    #[test]
    fn publishes_events_to_the_registered_listeners() {
        let received = Arc::new(AtomicUsize::new(0));
        let mut context = DefaultApplicationContext::default();

        context.add_application_listener(Box::new(CountingListener {
            received: Arc::clone(&received),
        }));
        // Registering another instance of the same type replaces the first one.
        context.add_application_listener(Box::new(CountingListener {
            received: Arc::clone(&received),
        }));

        context
            .publish_event(Box::new(TestEvent))
            .expect("publishing succeeds");
        assert_eq!(received.load(Ordering::SeqCst), 1);

        context.remove_application_listener(&DefaultApplicationContext::listener_id_of::<
            CountingListener,
        >());
        context
            .publish_event(Box::new(TestEvent))
            .expect("publishing succeeds");
        assert_eq!(received.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn falls_back_to_the_default_message() {
        let context = DefaultApplicationContext::default();

        assert!(context.message("missing", None, None).is_err());
        assert_eq!(
            context.message_or_default("missing", None, Some("fallback"), None),
            Some("fallback".to_owned())
        );
    }

    #[test]
    fn resolves_a_singleton_through_its_provider() {
        let mut context = DefaultApplicationContext::default();
        let provider: Provider<u32> = singleton(|_| 42_u32).into();
        context.register_provider(provider.into());

        assert_eq!(context.resolve::<u32>(), 42);

        // The instance is created once and kept by the context.
        assert!(context.contains_single::<u32>());
        assert_eq!(context.resolve::<u32>(), 42);
    }

    #[test]
    fn creates_a_transient_instance_for_every_resolution() {
        let created = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&created);

        let mut context = DefaultApplicationContext::default();
        let provider: Provider<u32> =
            transient(move |_| counter.fetch_add(1, Ordering::SeqCst) as u32).into();
        context.register_provider(provider.into());

        assert_eq!(context.resolve::<u32>(), 0);
        assert_eq!(context.resolve::<u32>(), 1);

        // A transient instance is not kept by the context.
        assert!(!context.contains_single::<u32>());
    }

    #[test]
    fn registers_the_providers_a_provider_is_bound_to() {
        let mut context = DefaultApplicationContext::default();
        let provider: Provider<u32> = singleton(|_| 7_u32)
            .name("seven")
            .bind(|value| value.to_string())
            .into();
        context.register_provider(provider.into());

        assert_eq!(context.resolve_with_name::<u32>("seven"), 7);
        assert_eq!(context.resolve_with_name::<String>("seven"), "7");
    }

    #[test]
    fn resolves_every_instance_of_a_type_through_its_providers() {
        let mut context = DefaultApplicationContext::default();
        let first: Provider<u32> = singleton(|_| 1_u32).name("one").into();
        let second: Provider<u32> = singleton(|_| 2_u32).name("two").into();
        context.register_provider(first.into());
        context.register_provider(second.into());

        let mut values = context.resolve_by_type::<u32>();
        values.sort();

        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    #[should_panic(expected = "circular dependency")]
    fn detects_a_circular_dependency() {
        let mut context = DefaultApplicationContext::default();
        let provider: Provider<u32> = singleton(|cx| {
            ApplicationContextExt::resolve_with_name::<u32>(cx, "loop")
        })
        .name("loop")
        .into();
        context.register_provider(provider.into());

        let _ = context.resolve_with_name::<u32>("loop");
    }
}



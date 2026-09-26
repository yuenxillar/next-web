//! Default [`ConfigurableApplicationContext`] implementation.
//!
//! [`DefaultApplicationContext`] is the context created by
//! `DefaultApplicationContextFactory` when an application does not install a
//! factory of its own. It assembles the pieces an application needs while it
//! starts up:
//!
//! - a [`ConfigurableEnvironment`] holding the properties and the profiles,
//! - a [`ResourceLoader`] reading the resources of the application,
//! - a singleton container ([`Context`]) holding the providers and the singletons,
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
//! Refreshing the context is also what populates it:
//!
//! 1. the providers an attribute macro submitted, such as the ones of
//!    `#[singleton]`, are loaded into the container,
//! 2. the providers that declare a condition are registered when their
//!    condition holds,
//! 3. the singletons whose provider asks for it are created eagerly, so that
//!    they exist before the application starts to serve. Every other singleton
//!    is created when it is first resolved.
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
    fmt::{self, Debug},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use next_web_context::{
    event::{ApplicationEventMulticaster, DefaultApplicationEventMulticaster},
    support::DelegatingMessageSource,
    ApplicationContext, ApplicationContextExt, ApplicationEvent, ApplicationEventPublisher,
    ApplicationListener, Definition, DynProvider, EagerCreateFunction, InstanceClone, Locale,
    MessageSource, MessageSourceResolvable, NoSuchMessageError, Scope,
    APPLICATION_ENVIRONMENT_SINGLETON_NAME, APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME,
    MESSAGE_SOURCE_SINGLETON_NAME, RESOURCE_LOADER_SINGLETON_NAME,
};
use next_web_core::{
    env::ConfigurableEnvironment,
    error::BoxError,
    io::{DefaultResourceLoader, ResourceLoader},
    metrics::{ApplicationStartup, DefaultApplicationStartup},
};
use next_web_singletons::factory::{
    config::SingletonRegistry,
    support::{DefaultListableSingletonFactory, DynSingle, Key},
    SingletonFactory,
};
use tracing::{enabled, Level};

use crate::{
    configurable_application_context::ContextError,
    next_web_application::APPLICATION_SHUTDOWN_HOOK, ApplicationEnvironment,
    ConfigurableApplicationContext,
};

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
    /// Resource loader reading the resources of the application.
    resource_loader: Option<Arc<dyn ResourceLoader>>,
    /// Collector recording the steps of the startup phase.
    application_startup: Box<dyn ApplicationStartup>,
    /// The singleton container holding the providers and the singletons.
    context: Context,
    /// Registered listeners, in registration order, paired with their identifier.
    listeners: Vec<(
        String,
        Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    )>,
    /// Message source used to resolve localized messages, when one is installed.
    message_source: Option<Arc<dyn MessageSource>>,
    /// Application event multicaster used to dispatch events to listeners.
    application_event_multicaster: Option<Arc<dyn ApplicationEventMulticaster>>,
    /// Lifecycle flags shared with the shutdown hook.
    lifecycle: Arc<LifecycleState>,
    /// Flag indicating whether the shutdown hook is registered.
    is_registered_shutdown_hook: AtomicBool,
}

impl DefaultApplicationContext {
    ///  Creates an empty application context with the given identifier.
    ///
    /// The environment and the startup collector are set to their defaults, and
    /// a unique identifier is generated.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier to install on the context.
    pub fn new(id: impl Into<String>) -> Self {
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

    /// Sets the loader used to read the resources of the application.
    ///
    /// The loader is registered as the `resourceLoader` singleton when the
    /// context is refreshed, so that the components of the framework that read
    /// resources (the message source, the banner, ...) resolve them the same
    /// way.
    ///
    /// # Arguments
    ///
    /// * `resource_loader` - The loader of the resources.
    pub fn set_resource_loader(&mut self, resource_loader: Arc<dyn ResourceLoader>) {
        self.resource_loader = Some(resource_loader);
    }

    /// Returns the loader used to read the resources of the application.
    ///
    /// # Returns
    ///
    /// The loader of the resources, which reads the resources directory of the
    /// application unless another one has been set.
    pub fn resource_loader(&self) -> Option<&Arc<dyn ResourceLoader>> {
        self.resource_loader.as_ref()
    }

    /// Sets whether a provider may replace another provider with the same key.
    ///
    /// # Arguments
    ///
    /// * `allow_override` - Whether replacing a provider is allowed.
    pub fn set_allow_override(&mut self, allow_override: bool) {
        self.context.allow_override = allow_override;
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
    pub fn register_provider(&mut self, provider: DynProvider) {
        self.context.load_provider(false, provider);
    }

    /// Registers every provider that was submitted by an attribute macro, such
    /// as `#[singleton]`.
    ///
    /// This is what makes the providers of the attribute macros available to the
    /// context without the application having to list them. The providers are
    /// loaded once, and [`ConfigurableApplicationContext::refresh`] does the
    /// same, so calling this method before refreshing the context is allowed.
    pub fn register_auto_providers(&mut self) {
        self.context.register_auto_providers();
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
        args: &[&dyn fmt::Display],
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

    /// Registers the providers whose condition holds.
    ///
    /// A provider that declares a condition is held back while the context is
    /// populated, and is only registered when its condition holds once the
    /// context is refreshed. The condition receives the context itself, so it
    /// can depend on the providers that are already registered.
    fn evaluate_conditional_providers(&mut self) {
        let mut pending = self.context.take_conditional_providers();
        // The providers are held back in registration order and popped from the
        // end, so they are reversed to evaluate them in that order.
        pending.reverse();

        while let Some((eager_create, provider)) = pending.pop() {
            let evaluate = provider
                .condition()
                .expect("a provider in the conditional providers has a condition");

            if evaluate(self) {
                self.context.load_provider(eager_create, provider);
            } else {
                tracing::warn!(
                    "the condition of a provider does not hold: {:?}",
                    provider.definition()
                );
            }
        }
    }

    /// Creates the instances of the providers that are eager.
    ///
    /// The functions are taken out of the container before they run, because
    /// they create their instance through the context, which owns the
    /// container.
    ///
    /// # Panics
    ///
    /// Panics when a provider asks for an asynchronous instance to be created
    /// eagerly, because a context resolves its providers synchronously.
    fn create_eager_instances(&mut self) {
        for (definition, create) in self.context.take_eager_create_functions() {
            match create {
                EagerCreateFunction::Sync(create) => create(self, definition.key.name),
                EagerCreateFunction::Async(_) => panic!(
                    "an asynchronous provider cannot be created eagerly by a synchronous context: {definition:?}"
                ),
                EagerCreateFunction::None => {
                    unreachable!("the container only records providers it has to create eagerly")
                }
            }
        }
    }

    /// Prepare this context for refreshing,
    /// setting its startup date and active flag as well as performing any initialization of property sources.
    fn prepare_refresh(&mut self) {
        // Switch to active.
        self.startup_date = current_time_millis();
        self.lifecycle.closed.store(false, Ordering::SeqCst);
        self.lifecycle.active.store(true, Ordering::SeqCst);

        if enabled!(Level::DEBUG) {
            tracing::debug!("Refreshing {}", self.id);
        }
    }

    /// Initialize the message source for this context.
    fn init_message_source(&mut self) -> Result<(), ContextError> {
        if let Some(message_source) = self
            .singleton_factory()?
            .registry_mut()
            .get_singleton::<Arc<dyn MessageSource>, &'static str>(MESSAGE_SOURCE_SINGLETON_NAME)
            .map(Clone::clone)
        {
            tracing::trace!("Using MessageSource [{:?}]", message_source);
            self.message_source = Some(message_source);
        } else {
            let parent_message_source = self
                .parent
                .as_ref()
                .map(Clone::clone)
                .map(|ctx| ctx as Arc<dyn MessageSource>);
            let dms = Arc::new(DelegatingMessageSource::new(parent_message_source))
                as Arc<dyn MessageSource>;
            self.singleton_factory()?
                .registry_mut()
                .register_singleton(MESSAGE_SOURCE_SINGLETON_NAME, Arc::clone(&dms));

            tracing::trace!(
                "No '{MESSAGE_SOURCE_SINGLETON_NAME}' singleton, using [{:?}]",
                dms
            );
            self.message_source = Some(dms);
        }

        Ok(())
    }

    /// Initialize the ApplicationEventMulticaster.
    ///
    /// A multicaster the application installed under
    /// [`APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME`] wins; otherwise the
    /// default one of the framework is used and registered under that name, so
    /// that components can publish events through it. Either way the listeners
    /// of the context are added to it:
    ///
    /// 1. the listeners the providers of the application contribute, which are
    ///    resolved by their type,
    /// 2. the listeners that were registered through
    ///    [`ConfigurableApplicationContext::add_application_listener`], which
    ///    win over the listeners of a provider of the same concrete type.
    fn init_application_event_multicaster(&mut self) -> Result<(), ContextError> {
        let installed = self
            .singleton_factory()?
            .registry_mut()
            .get_singleton::<Arc<dyn ApplicationEventMulticaster>, &'static str>(
                APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME,
            )
            .map(Clone::clone);

        let event_multicaster = match installed {
            Some(event_multicaster) => {
                tracing::trace!(
                    "Using ApplicationEventMulticaster [{:?}]",
                    event_multicaster
                );
                event_multicaster
            }
            None => {
                // A multicaster that was created before the context was
                // refreshed is reused, so the listeners that were registered on
                // it and the events it already received stay with the same
                // multicaster.
                let event_multicaster = match self.application_event_multicaster.as_ref() {
                    Some(event_multicaster) => Arc::clone(event_multicaster),
                    None => {
                        let event_multicaster = self.create_application_event_multicaster();
                        self.application_event_multicaster = Some(Arc::clone(&event_multicaster));
                        event_multicaster
                    }
                };

                self.singleton_factory()?.registry_mut().register_singleton(
                    APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME,
                    Arc::clone(&event_multicaster),
                );

                tracing::trace!(
                    "No '{APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME}' singleton, using [{:?}]",
                    event_multicaster
                );

                event_multicaster
            }
        };

        self.application_event_multicaster = Some(Arc::clone(&event_multicaster));

        // Listeners contributed by the providers of the application.
        let listeners =
            self.resolve_by_type::<Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>>();
        for listener in listeners {
            let id = Self::listener_id(listener.as_ref().type_id());
            event_multicaster.add_application_listener(id, listener);
        }

        // Statically specified listeners.
        for (id, listener) in self.listeners.iter() {
            event_multicaster.add_application_listener(id.to_owned(), Arc::clone(listener));
        }

        Ok(())
    }

    /// Creates the multicaster of the context and adds the listeners that are
    /// registered in it.
    ///
    /// The multicaster is the default one of the framework, which dispatches
    /// the events inline and logs the failures of its listeners.
    fn create_application_event_multicaster(&self) -> Arc<dyn ApplicationEventMulticaster> {
        let event_multicaster = Arc::new(DefaultApplicationEventMulticaster::default())
            as Arc<dyn ApplicationEventMulticaster>;

        for (id, listener) in self.listeners.iter() {
            event_multicaster.add_application_listener(id.to_owned(), Arc::clone(listener));
        }

        event_multicaster
    }

    /// Called when the application context is refreshed.
    fn on_refresh(&mut self) {
        self.insert_singleton_with_name::<Arc<dyn ConfigurableEnvironment>>(
            Arc::clone(&self.environment),
            APPLICATION_ENVIRONMENT_SINGLETON_NAME,
        );

        // A loader the application configured wins over the shared one, which is
        // loaded once per process: the context and the components that read the
        // resources directly, such as the banner, therefore share the resources
        // they hold.
        let resource_loader: Arc<dyn ResourceLoader> = match self.resource_loader.as_ref() {
            Some(resource_loader) => Arc::clone(resource_loader),
            None => DefaultResourceLoader::shared_arc(),
        };

        self.insert_singleton_with_name::<Arc<dyn ResourceLoader>>(
            resource_loader,
            RESOURCE_LOADER_SINGLETON_NAME,
        );
    }
}

impl Default for DefaultApplicationContext {
    fn default() -> Self {
        Self {
            id: String::from("application"),
            application_name: String::new(),
            startup_date: current_time_millis(),
            parent: None,
            environment: Arc::new(ApplicationEnvironment::default()),
            resource_loader: None,
            application_startup: Box::new(DefaultApplicationStartup::default()),
            context: Context::default(),
            listeners: Vec::new(),
            message_source: None,
            application_event_multicaster: None,
            lifecycle: Arc::new(LifecycleState::default()),
            is_registered_shutdown_hook: AtomicBool::new(false),
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
        listener: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    ) {
        let id = Self::listener_id(listener.as_ref().type_id());

        // A concrete listener type is registered at most once. Adding another
        // instance of the same type replaces the previous one, which mirrors the
        // set semantics of the original Spring implementation.
        match self.listeners.iter_mut().find(|(key, _)| *key == id) {
            Some((_, registered)) => *registered = Arc::clone(&listener),
            None => self.listeners.push((id.clone(), Arc::clone(&listener))),
        }

        // A listener that is registered while the context is active is added to
        // the multicaster right away, so that it receives the events a
        // component publishes from now on. Before the context is refreshed the
        // multicaster is created on demand, which is what lets an application
        // publish an event without refreshing its context first.
        match self.application_event_multicaster.as_ref() {
            Some(multicaster) => multicaster.add_application_listener(id, listener),
            None => {
                let multicaster = self.create_application_event_multicaster();
                multicaster.add_application_listener(id, listener);
                self.application_event_multicaster = Some(multicaster);
            }
        }
    }

    fn remove_application_listener(&mut self, id: &str) {
        self.listeners.retain(|(listener_id, _)| listener_id != id);

        if let Some(multicaster) = self.application_event_multicaster.as_ref() {
            multicaster.remove_application_listener(id.to_owned());
        }
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

        let started = std::time::Instant::now();
        let mut context_refresh = self.application_startup.start("next.context.refresh");

        // Prepare this context for refreshing.
        self.prepare_refresh();

        // Initialize message source for this context.
        self.init_message_source()?;

        // Initialize event multicaster for this context.
        self.init_application_event_multicaster()?;

        // Initialize other special instances
        self.on_refresh();

        // The providers the attribute macros submitted, such as the singleton
        // providers of `#[singleton]`, are loaded first, so that the singletons
        // of the application are known to the context.
        self.context.register_auto_providers();
        // A provider that declares a condition is only registered when its
        // condition holds.
        self.evaluate_conditional_providers();
        // The singletons that have to exist eagerly are created while
        // refreshing, mirroring the "finishRefresh" phase of the original
        // context. Every other singleton is created when it is resolved.
        self.create_eager_instances();
        // Last chance for the container to install the singletons it considers
        // essential.
        self.context.singleton_factory_mut().initialize_defaults();
        context_refresh.end();

        tracing::info!(
            "Application context refreshed elapsed: {:?}",
            started.elapsed()
        );

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
        if self.is_registered_shutdown_hook.load(Ordering::SeqCst) {
            return;
        }

        let lifecycle = Arc::clone(&self.lifecycle);
        let shutdown_hook = APPLICATION_SHUTDOWN_HOOK.get_or_init(|| Default::default());
        shutdown_hook.add_hook(move || async move {
            // The hook cannot borrow the context, so it only marks it as closed;
            // `close` remains responsible for releasing the singletons.
            lifecycle.active.store(false, Ordering::SeqCst);
            lifecycle.closed.store(true, Ordering::SeqCst);
        });

        self.is_registered_shutdown_hook
            .store(true, Ordering::SeqCst);
    }

    fn close(&mut self) -> Result<(), ContextError> {
        if self.is_closed() {
            // Closing is idempotent.
            return Ok(());
        }

        // Release the singletons held by this context. The parent context is
        // deliberately left untouched, because it has its own lifecycle.
        self.context.clear_all_singletons();
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
        Ok(self.context.singleton_factory_mut())
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
        self.context
            .singleton_factory_mut()
            .registry_mut()
            .insert_dyn(key, DynSingle::new(instance, clone));
    }

    fn contains_singleton_boxed(&self, key: &Key) -> bool {
        self.context
            .singleton_factory()
            .registry()
            .contains_key(key)
    }

    fn get_singleton_boxed(&self, key: &Key) -> Option<&(dyn Any + Send + Sync)> {
        self.context
            .singleton_factory()
            .registry()
            .get_dyn(key)
            .map(DynSingle::as_any)
    }

    fn get_singleton_boxed_mut(&mut self, key: &Key) -> Option<&mut (dyn Any + Send + Sync)> {
        self.context
            .singleton_factory_mut()
            .registry_mut()
            .get_dyn_mut(key)
            .map(DynSingle::as_any_mut)
    }

    fn remove_singleton_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        self.context
            .singleton_factory_mut()
            .registry_mut()
            .remove_dyn(key)
            .map(DynSingle::into_inner_boxed)
    }

    fn resolve_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>> {
        // An instance that was created before is handed out as it is.
        if let Some(instance) = self.context.singleton_factory().registry().get_dyn(key) {
            return instance.get_owned_boxed();
        }

        let provider = self.context.provider(key)?.clone();
        let definition = provider.definition().clone();
        let scope = definition.scope;

        if scope == Scope::SingleOwner {
            // A single owner instance belongs to the context, so it can only be
            // borrowed from it once it was created.
            panic!("a single owner instance cannot be resolved as an owned value: {definition:?}");
        }

        if self.context.is_resolving(key) {
            panic!("a circular dependency was detected while resolving: {definition:?}");
        }

        self.context.push_dependency_chain(key.clone());
        let instance = provider.build_boxed(self);
        self.context.pop_dependency_chain();

        if scope != Scope::Singleton {
            // A transient instance is created for every resolution and is not
            // stored in the context.
            return Some(instance);
        }

        let erased_clone: Option<InstanceClone> = provider.clone_instance().map(|clone| {
            let origin = provider.origin();

            let erased: InstanceClone = Arc::new(move |instance: &(dyn Any + Send + Sync)| {
                clone(origin.as_ref(), instance)
            });

            erased
        });

        match erased_clone {
            Some(clone) => {
                // Keep one copy in the context and return the other one.
                let stored = clone(instance.as_ref());
                self.context
                    .singleton_factory_mut()
                    .registry_mut()
                    .insert_dyn(
                        key.clone(),
                        DynSingle::new(stored, Some(Arc::clone(&clone))),
                    );

                Some(instance)
            }
            None => {
                // The instance cannot be copied, so it can only be borrowed from
                // the context afterwards.
                self.context
                    .singleton_factory_mut()
                    .registry_mut()
                    .insert_dyn(key.clone(), DynSingle::new(instance, None));

                None
            }
        }
    }

    fn resolve_all_of_type(&mut self, ty: TypeId) -> Vec<Box<dyn Any + Send + Sync>> {
        let keys = self.keys_of_type(ty);

        keys.iter()
            .filter_map(|key| self.resolve_boxed(key))
            .collect()
    }

    fn keys_of_type(&self, ty: TypeId) -> Vec<Key> {
        // An instance either comes from a provider, or was inserted into the
        // context directly.
        let mut keys: Vec<Key> = self.context.keys_of_type(ty);
        keys.extend(self.context.singleton_factory().registry().keys_of_type(ty));
        keys.sort();
        keys.dedup();
        keys
    }
}

impl ApplicationEventPublisher for DefaultApplicationContext {
    fn publish_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), BoxError> {
        // Multicast the event to the application event multicaster, if one is configured.
        if let Some(multicaster) = self.application_event_multicaster.as_ref() {
            multicaster.multicast_event(event.clone())?;
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
        args: &[&dyn fmt::Display],
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
            if let Ok(message) =
                self.resolve_message(code, arguments.as_deref().unwrap_or_default(), locale)
            {
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
        args: &[&dyn fmt::Display],
        default_message: Option<&str>,
        locale: Option<&Locale>,
    ) -> Option<String> {
        self.resolve_message(code, args, locale)
            .ok()
            .or_else(|| default_message.map(|message| message.to_owned()))
    }
}

impl Debug for DefaultApplicationContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultApplicationContext")
            .field("id", &self.id)
            .field("application_name", &self.application_name)
            .field("startup_date", &self.startup_date)
            .field("parent", &self.parent)
            .field("environment", &"")
            .field("application_startup", &"")
            .field("context", &"")
            .field("listeners", &"")
            .field("message_source", &self.message_source)
            .field("lifecycle", &self.lifecycle)
            .field(
                "is_registered_shutdown_hook",
                &self.is_registered_shutdown_hook,
            )
            .finish()
    }
}

/// Returns the current wall clock time in milliseconds since the Unix epoch.
fn current_time_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// The singleton container of an application context.
///
/// The container is the `Context` of the original dependency injection
/// implementation, ported so that a [`DefaultApplicationContext`] owns its
/// providers and its singletons in one place:
///
/// - the providers are the instances of `#[singleton]`, `#[transient]` and
///   `#[singleowner]`, plus everything an application registers by hand,
/// - the singletons are the instances the providers created, held by a
///   [`DefaultListableSingletonFactory`],
/// - the dependency chain is what detects a cycle while an instance is built,
/// - the eager create functions are the ones a refresh has to run, and the
///   conditional providers are the ones whose condition has to be evaluated
///   first.
struct Context {
    /// The providers of the application, keyed by the instance they create.
    providers: HashMap<Key, DynProvider>,
    /// The factory holding the singletons the providers created.
    singleton_factory: DefaultListableSingletonFactory,
    /// The keys of the providers that are being created, innermost last.
    ///
    /// A key that appears twice in the chain is a dependency cycle, which is
    /// what stops a provider from resolving itself forever.
    dependency_chain: Vec<Key>,
    /// The providers whose condition has not been evaluated yet, together with
    /// whether their instances have to be created eagerly.
    conditional_providers: Vec<(bool, DynProvider)>,
    /// The functions creating the instances of the eager providers.
    eager_create_functions: Vec<(Definition, EagerCreateFunction)>,
    /// Whether a provider may replace another provider with the same key.
    allow_override: bool,
    /// Whether every provider is created eagerly, unless a provider says
    /// otherwise.
    eager_create: bool,
    /// Whether only a single instance may be created eagerly.
    ///
    /// A transient instance is created for every resolution, so creating one
    /// eagerly would have no effect. The flag therefore only allows the
    /// [`Scope::Singleton`] and [`Scope::SingleOwner`] scopes until it is
    /// cleared.
    allow_only_single_eager_create: bool,
    /// Whether the providers the attribute macros submitted have been loaded.
    auto_providers_loaded: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            singleton_factory: DefaultListableSingletonFactory::default(),
            dependency_chain: Vec::new(),
            conditional_providers: Vec::new(),
            eager_create_functions: Vec::new(),
            allow_override: true,
            eager_create: false,
            allow_only_single_eager_create: true,
            auto_providers_loaded: false,
        }
    }
}

impl Context {
    /// Registers a provider, together with the providers it is bound to.
    ///
    /// # Panics
    ///
    /// Panics when the key of the provider is already registered and the
    /// container does not allow a provider to be overridden.
    ///
    /// # Arguments
    ///
    /// * `eager_create` - Whether the instance of the provider has to be
    ///   created while the context is refreshed, in addition to the providers
    ///   that ask for it themselves.
    /// * `provider` - The provider to register.
    #[track_caller]
    fn load_provider(&mut self, eager_create: bool, mut provider: DynProvider) {
        // A provider that binds a type registers one provider per bound type.
        if let Some(bindings) = provider.binding_providers() {
            for binding in bindings {
                self.load_provider(eager_create, binding);
            }
        }

        let definition = provider.definition().clone();
        let need_eager_create = self.eager_create || eager_create || provider.eager_create();
        let allow_eager_create = !self.allow_only_single_eager_create
            || matches!(definition.scope, Scope::Singleton | Scope::SingleOwner);

        if need_eager_create && allow_eager_create {
            self.eager_create_functions
                .push((definition.clone(), provider.eager_create_function()));
        }

        let key = definition.key.clone();
        if self.providers.contains_key(&key) && !self.allow_override {
            panic!("a provider with the same key is already registered: {definition:?}");
        }

        self.providers.insert(key, provider);
    }

    /// Registers the given providers, together with the providers they are
    /// bound to.
    ///
    /// A provider that declares a condition is not registered until the
    /// condition is evaluated, which happens while the context is refreshed.
    ///
    /// # Arguments
    ///
    /// * `eager_create` - Whether the instances of the providers have to be
    ///   created while the context is refreshed.
    /// * `providers` - The providers to register.
    fn load_providers(&mut self, eager_create: bool, providers: Vec<DynProvider>) {
        for provider in providers {
            if provider.condition().is_some() {
                self.conditional_providers.push((eager_create, provider));
                continue;
            }

            self.load_provider(eager_create, provider);
        }
    }

    /// Registers every provider that an attribute macro submitted.
    ///
    /// This is what makes the singletons an application declares with
    /// `#[singleton]` available, without the application having to list them.
    /// The providers are loaded once: a second call is a no-op, so refreshing
    /// the context does not register them twice.
    fn register_auto_providers(&mut self) {
        if self.auto_providers_loaded {
            return;
        }
        self.auto_providers_loaded = true;

        let providers: Vec<DynProvider> = next_web_context::auto_registered_providers().collect();
        self.load_providers(false, providers);
    }

    /// Returns the provider registered under the given key, if any.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the provider.
    fn provider(&self, key: &Key) -> Option<&DynProvider> {
        self.providers.get(key)
    }

    /// Returns the keys of the providers whose instance has the given type.
    ///
    /// # Arguments
    ///
    /// * `ty` - The [`TypeId`] of the instances to look for.
    fn keys_of_type(&self, ty: TypeId) -> Vec<Key> {
        self.providers
            .keys()
            .filter(|key| key.ty.id == ty)
            .cloned()
            .collect()
    }

    /// Returns the singletons the providers created.
    fn singleton_factory(&self) -> &DefaultListableSingletonFactory {
        &self.singleton_factory
    }

    /// Returns the singletons the providers created.
    fn singleton_factory_mut(&mut self) -> &mut DefaultListableSingletonFactory {
        &mut self.singleton_factory
    }

    /// Returns whether the given key is being resolved.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look for in the dependency chain.
    fn is_resolving(&self, key: &Key) -> bool {
        self.dependency_chain.contains(key)
    }

    /// Pushes the key of a provider that is about to be created.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the provider.
    fn push_dependency_chain(&mut self, key: Key) {
        self.dependency_chain.push(key);
    }

    /// Pops the key of the provider that was created last.
    ///
    /// # Panics
    ///
    /// Panics when the chain is empty, which cannot happen because a key is
    /// pushed before a provider is created.
    fn pop_dependency_chain(&mut self) {
        self.dependency_chain
            .pop()
            .expect("a key is pushed before a provider is created");
    }

    /// Returns the providers whose condition has not been evaluated yet.
    fn take_conditional_providers(&mut self) -> Vec<(bool, DynProvider)> {
        std::mem::take(&mut self.conditional_providers)
    }

    /// Returns the functions creating the instances of the eager providers.
    ///
    /// The functions are returned in the order the providers were registered,
    /// and are removed from the container, so that refreshing the context runs
    /// each of them once.
    fn take_eager_create_functions(&mut self) -> Vec<(Definition, EagerCreateFunction)> {
        std::mem::take(&mut self.eager_create_functions)
    }

    /// Releases the singletons the providers created.
    fn clear_all_singletons(&mut self) {
        self.singleton_factory.clear_all_singletons();
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::AtomicUsize;

    use next_web_context::{singleton, transient, ApplicationContextExt, Provider};
    use next_web_singletons::factory::config::SingletonRegistry;

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

        context.add_application_listener(Arc::new(CountingListener {
            received: Arc::clone(&received),
        }));
        // Registering another instance of the same type replaces the first one.
        context.add_application_listener(Arc::new(CountingListener {
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

        assert!(context.message("missing", &[], None).is_err());
        assert_eq!(
            context.message_or_default("missing", &[], Some("fallback"), None),
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
        assert!(context.contains_singleton::<u32>());
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
        assert!(!context.contains_singleton::<u32>());
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
        let provider: Provider<u32> =
            singleton(|cx| ApplicationContextExt::resolve_with_name::<u32>(cx, "loop"))
                .name("loop")
                .into();
        context.register_provider(provider.into());

        let _ = context.resolve_with_name::<u32>("loop");
    }

    /// A singleton that the attribute macro registers automatically.
    #[crate::macros::bind::singleton]
    #[derive(Clone)]
    struct AutoRegisteredService;

    /// A singleton that the attribute macro marks as eager.
    #[crate::macros::bind::singleton(eager_create = true)]
    #[derive(Clone)]
    struct AutoRegisteredEagerService;

    #[test]
    fn creates_the_singleton_that_the_attribute_macro_registered() {
        let mut context = DefaultApplicationContext::default();
        context.refresh().expect("refreshing succeeds");

        // The provider the macro registered has to be known to the context, so
        // that the instance can be created and is then held by the context.
        context.just_create_singleton_with_name::<AutoRegisteredService>(
            next_web_context::default_singleton_name::<AutoRegisteredService>(),
        );

        assert!(context.contains_singleton_with_default_name::<AutoRegisteredService>());
    }

    #[test]
    fn creates_the_eager_singleton_while_refreshing() {
        let mut context = DefaultApplicationContext::default();
        context.refresh().expect("refreshing succeeds");

        assert!(context.contains_singleton_with_default_name::<AutoRegisteredEagerService>());
    }

    /// A trait whose implementations are bound to it, the way the framework
    /// binds an application runner or a filter to a trait object.
    trait Greeter: Send + Sync {
        fn greet(&self) -> String;
    }

    #[crate::macros::bind::singleton(binds = [Self::into_greeter])]
    #[derive(Clone)]
    struct EnglishGreeter;

    impl Greeter for EnglishGreeter {
        fn greet(&self) -> String {
            "hello".to_owned()
        }
    }

    impl EnglishGreeter {
        fn into_greeter(self) -> Arc<dyn Greeter> {
            Arc::new(self)
        }
    }

    #[test]
    fn resolves_the_bindings_of_an_auto_registered_singleton() {
        let mut context = DefaultApplicationContext::default();
        context.refresh().expect("refreshing succeeds");

        // The binding provider is registered together with the provider of the
        // type it is bound to, so the bound instances can be resolved by type.
        let greeters = context.resolve_by_type::<Arc<dyn Greeter>>();

        assert_eq!(greeters.len(), 1);
        assert_eq!(greeters[0].greet(), "hello");
    }

    #[test]
    fn creates_and_borrows_every_instance_of_a_type() {
        let mut context = DefaultApplicationContext::default();
        context.refresh().expect("refreshing succeeds");

        let name = next_web_context::default_singleton_name::<AutoRegisteredService>();

        // The provider of the macro is known, so the instance is created on
        // demand and the context holds it afterwards.
        assert!(context.try_just_create_singleton_with_name::<AutoRegisteredService>(name.clone()));
        assert!(!context.try_just_create_singleton_with_name::<u32>("missing"));

        // Every instance of the type is reported once it was created.
        assert_eq!(
            context
                .get_singletons_by_type::<AutoRegisteredService>()
                .len(),
            1
        );
        assert_eq!(
            context.try_just_create_singletons_by_type::<AutoRegisteredService>(),
            vec![true]
        );
    }

    #[test]
    fn removing_a_singleton_keeps_the_provider_that_created_it() {
        let mut context = DefaultApplicationContext::default();
        context.refresh().expect("refreshing succeeds");

        let name = next_web_context::default_singleton_name::<AutoRegisteredService>();
        context.just_create_singleton_with_name::<AutoRegisteredService>(name.clone());

        assert!(context
            .remove_singleton_with_name::<AutoRegisteredService>(name.clone())
            .is_some());
        assert!(!context.contains_singleton_with_name::<AutoRegisteredService>(name.clone()));

        // Only the instance was removed, so the context creates it again.
        context.just_create_singleton_with_name::<AutoRegisteredService>(name.clone());
        assert!(context.contains_singleton_with_name::<AutoRegisteredService>(name));
    }
}

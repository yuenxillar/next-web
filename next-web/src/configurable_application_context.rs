//! SPI trait to be implemented by application contexts.
//!
//! Provides facilities to configure an application context in addition to the
//! application context client methods in the [`ApplicationContext`] trait.
//!
//! Configuration and lifecycle methods are encapsulated here to avoid making
//! them obvious to client code. These methods should only be used by startup
//! and shutdown code.

use std::sync::Arc;

use next_web_context::{ApplicationContext, ApplicationEvent, ApplicationListener};
use next_web_core::{env::ConfigurableEnvironment, metrics::ApplicationStartup};
use next_web_singletons::factory::{
    support::DefaultListableSingletonFactory, ListableSingletonFactory,
};

/// Any number of these characters are considered delimiters between multiple
/// context config paths in a single string value.
pub const CONFIG_LOCATION_DELIMITERS: &str = ",; \t\n";

/// The name of the bootstrap executor bean in the context.
///
/// If none is supplied, no background bootstrapping will be active.
pub const BOOTSTRAP_EXECUTOR_BEAN_NAME: &str = "bootstrapExecutor";

/// Name of the `ConversionService` bean in the factory.
///
/// If none is supplied, default conversion rules apply.
pub const CONVERSION_SERVICE_BEAN_NAME: &str = "conversionService";

/// Name of the `LoadTimeWeaver` bean in the factory.
///
/// If such a bean is supplied, the context will use a temporary class loader
/// for type matching, in order to allow the `LoadTimeWeaver` to process all
/// actual bean classes.
pub const LOAD_TIME_WEAVER_BEAN_NAME: &str = "loadTimeWeaver";

/// Name of the environment bean in the factory.
pub const ENVIRONMENT_BEAN_NAME: &str = "environment";

/// Name of the system properties bean in the factory.
pub const SYSTEM_PROPERTIES_BEAN_NAME: &str = "systemProperties";

/// Name of the operating system environment bean in the factory.
pub const SYSTEM_ENVIRONMENT_BEAN_NAME: &str = "systemEnvironment";

/// Name of the application startup bean in the factory.
pub const APPLICATION_STARTUP_BEAN_NAME: &str = "applicationStartup";

/// Name of the shutdown hook thread.
pub const SHUTDOWN_HOOK_THREAD_NAME: &str = "SpringContextShutdownHook";

/// Errors returned by [`ConfigurableApplicationContext`] operations.
#[derive(Debug)]
pub enum ContextError {
    /// Returned when the bean factory could not be initialized.
    Beans(Box<dyn std::error::Error + Send + Sync>),
    /// Returned when an operation is attempted in an invalid state.
    IllegalState(String),
    /// Returned for I/O failures while closing the context.
    Io(std::io::Error),
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextError::Beans(error) => write!(f, "bean error: {}", error),
            ContextError::IllegalState(message) => write!(f, "illegal state: {}", message),
            ContextError::Io(error) => write!(f, "I/O error: {}", error),
        }
    }
}

impl std::error::Error for ContextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ContextError::Beans(error) => Some(error.as_ref()),
            ContextError::IllegalState(_) => None,
            ContextError::Io(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for ContextError {
    fn from(error: std::io::Error) -> Self {
        ContextError::Io(error)
    }
}

/// SPI trait to be implemented by application contexts.
///
/// # Purpose
///
/// Provides facilities to configure an application context in addition to the
/// client methods in [`ApplicationContext`].
///
/// # Lifecycle
///
/// Configuration and lifecycle methods are encapsulated here to avoid making
/// them obvious to client code. These methods should only be used by startup
/// and shutdown code.
///
/// # Supertraits
///
/// This trait extends [`ApplicationContext`], [`Lifecycle`], and is expected to
/// behave like a closeable resource. In Rust, `Closeable` maps to
/// [`Self::close`] returning a [`Result`].
pub trait ConfigurableApplicationContext<F = DefaultListableSingletonFactory>
where
    Self: ApplicationContext,
    Self: Send + Sync,
    F: ListableSingletonFactory,
{
    /// Sets the unique ID of this application context.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier.
    fn set_id(&mut self, id: String);

    /// Sets the parent of this application context.
    ///
    /// Note that the parent should not be changed after construction. It should
    /// only be set outside a constructor if it is not available when the
    /// context is created, for example during web application context setup.
    ///
    /// # Arguments
    ///
    /// * `parent` - The optional parent context.
    fn set_parent(&mut self, parent: Option<Arc<dyn ApplicationContext>>);

    /// Sets the [`ConfigurableEnvironment`] for this application context.
    ///
    /// # Arguments
    ///
    /// * `environment` - The new environment.
    fn set_environment(&mut self, environment: Box<dyn ConfigurableEnvironment>);

    /// Returns the [`ConfigurableEnvironment`] for this application context in
    /// configurable form, allowing for further customization.
    ///
    /// # Returns
    ///
    /// The configurable environment.
    fn environment(&self) -> &dyn ConfigurableEnvironment;

    /// Sets the [`ApplicationStartup`] for this application context.
    ///
    /// This allows the application context to record metrics during startup.
    ///
    /// # Arguments
    ///
    /// * `application_startup` - The new startup metrics collector.
    fn set_application_startup(&mut self, application_startup: Box<dyn ApplicationStartup>);

    /// Returns the [`ApplicationStartup`] for this application context.
    ///
    /// # Returns
    ///
    /// The startup metrics collector.
    fn application_startup(&self) -> &dyn ApplicationStartup;

    /// Adds a new [`ApplicationListener`] that will be notified on context
    /// events such as context refresh and context shutdown.
    ///
    /// Any listener registered here will be applied on refresh if the context
    /// is not active yet, or on the fly with the current event multicaster if
    /// the context is already active.
    ///
    /// # Arguments
    ///
    /// * `listener` - The listener to register.
    fn add_application_listener(
        &mut self,
        listener: Box<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    );

    /// Removes the given [`ApplicationListener`] from this context's set of
    /// listeners.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the listener to remove.
    fn remove_application_listener(&mut self, id: &str);

    /// Loads or refreshes the persistent representation of the configuration.
    ///
    /// The configuration might come from code-based configuration, a
    /// configuration file, a properties file, a relational database schema, or
    /// some other format.
    ///
    /// As this is a startup method, it should destroy already created
    /// singletons if it fails, to avoid dangling resources. After invocation of
    /// this method, either all or no singletons should be instantiated.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::Beans`] if the bean factory could not be
    /// initialized, or [`ContextError::IllegalState`] if the context is already
    /// initialized and multiple refresh attempts are not supported.
    fn refresh(&mut self) -> Result<(), ContextError>;

    /// Pauses all beans in this application context if necessary, and
    /// subsequently restarts all auto-startup beans, effectively restoring the
    /// lifecycle state after [`Self::refresh`].
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    fn restart(&mut self) -> Result<(), ContextError>;

    /// Stops all beans in this application context unless they explicitly opt
    /// out of pausing.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    fn pause(&mut self) -> Result<(), ContextError>;

    /// Registers a shutdown hook with the runtime, closing this context on
    /// shutdown unless it has already been closed at that time.
    ///
    /// This method can be called multiple times. Only one shutdown hook will be
    /// registered for each context instance.
    fn register_shutdown_hook(&mut self);

    /// Closes this application context, releasing all resources and locks that
    /// the implementation might hold, including destroying all cached singleton
    /// beans.
    ///
    /// Note: does *not* invoke `close` on a parent context; parent contexts
    /// have their own, independent lifecycle.
    ///
    /// This method can be called multiple times without side effects.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    fn close(&mut self) -> Result<(), ContextError>;

    /// Returns whether this context has been closed already.
    ///
    /// This does not indicate whether context shutdown has completed. Use
    /// [`Self::is_active`] for differentiating between those scenarios: a
    /// context becomes inactive once it has been fully shut down and the
    /// original [`Self::close`] call has returned.
    ///
    /// # Returns
    ///
    /// `true` if the context has been closed.
    fn is_closed(&self) -> bool;

    /// Determines whether this application context is active, that is, whether
    /// it has been refreshed at least once and has not been closed yet.
    ///
    /// # Returns
    ///
    /// `true` if the context is still active.
    fn is_active(&self) -> bool;

    /// Return the parent context, or none if there is no parent and this is the root of the context hierarchy.
    fn parent(&self) -> Option<&dyn ApplicationContext>;

    /// Returns the internal bean factory of this application context.
    ///
    /// The internal factory is generally only accessible while the context is
    /// active, that is, in-between [`Self::refresh`] and [`Self::close`]. The
    /// [`Self::is_active`] flag can be used to check whether the context is in
    /// an appropriate state.
    ///
    /// # Returns
    ///
    /// The underlying bean factory.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError::IllegalState`] if the context does not hold an
    /// internal bean factory (usually if [`Self::refresh`] has not been called
    /// yet or if [`Self::close`] has already been called).
    fn singleton_factory(&mut self) -> Result<&mut F, ContextError>;
}

use std::{
    any::{type_name_of_val, Any},
    collections::{HashMap, HashSet},
    error::Error,
    future::Future,
    panic::AssertUnwindSafe,
    sync::{Arc, LazyLock},
};

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::{response::IntoResponse, Router};
use next_web_core::{
    anys::any_value::AnyValue,
    constants::application_constants::APPLICATION_DEFAULT_PORT,
    env::{
        CompositePropertySource, ConfigurableEnvironment, MutablePropertySources, PropertySource,
        SimpleCommandLinePropertySource, COMMAND_LINE_PROPERTY_SOURCE_NAME,
        DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME,
    },
    io::{DefaultResourceLoader, ResourceLoader},
    metrics::{ApplicationStartup, DefaultApplicationStartup, StartupStep},
    util::indexmap::IndexMap,
};
use next_web_singletons::factory::{
    config::SingletonRegistry, ListableSingletonFactory, SingletonFactory,
};
use reqwest::StatusCode;
use tracing::{enabled, Level};

use crate::{
    application_banner_printer::PrintedBanner,
    banner::BannerMode,
    context::{logging::LoggingEventHandler, properties::source::ConfigurationPropertySources},
    diagnostics::error_analyzers::ErrorAnalyzers,
    env::{DefaultPropertiesPropertySource, MapPropertySource},
    support::EnvironmentPostProcessorEventHandler,
    web::server::{Server, WebServer},
    ApplicationArguments, ApplicationBannerPrinter, ApplicationContextFactory,
    ApplicationContextInitializer, ApplicationEnvironment, ApplicationEventHandler,
    ApplicationInfoPropertySource, ApplicationProperties, ApplicationRunner,
    ApplicationShutdownHook, Banner, ConfigurableApplicationContext, DefaultApplicationArguments,
    DefaultApplicationContextFactory, ErrorHandler, Event, NextWebErrorReporter, StartupInfoLogger,
};

/// Loader used when the application does not configure one of its own.
///
/// It is created once, so that a banner resolved through it stays valid for as
/// long as it is borrowed.
static DEFAULT_RESOURCE_LOADER: LazyLock<DefaultResourceLoader> =
    LazyLock::new(DefaultResourceLoader::default);

type ApplicationResult<T> = Result<T, Box<dyn Error>>;

pub trait Application<E = ()>
where
    E: ErrorHandler,
{
    /// Before starting the application
    #[allow(unused_variables)]
    fn on_ready(
        &self,
        ctx: &mut dyn ConfigurableApplicationContext,
    ) -> impl Future<Output = ApplicationResult<()>> {
        std::future::ready(Ok(()))
    }

    fn router(&self, ctx: &mut dyn ConfigurableApplicationContext) -> Router {
        Router::new()
    }

    fn open_api(&self, ctx: &mut dyn ConfigurableApplicationContext) {}

    fn fallback() -> impl IntoResponse {
        let mut resp = E::handle_error("Not Found").into_response();
        *resp.status_mut() = StatusCode::NOT_FOUND;
        resp
    }

    fn catch_panic(err: Box<dyn Any + Send + 'static>) -> impl IntoResponse {
        let error = err
            .downcast_ref::<String>()
            .map(String::as_str)
            .unwrap_or_else(|| {
                err.downcast_ref::<&str>()
                    .map(|s| *s)
                    .unwrap_or("Unknown error")
            });

        tracing::error!("Service panicked: {}", error);
        let mut resp = E::handle_error(error).into_response();
        *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        resp
    }
}

pub struct NextWebApplication<T> {
    add_command_line_properties: bool,
    banner: Option<Box<dyn Banner>>,
    resource_loader: Option<Arc<dyn ResourceLoader>>,
    environment: Option<Box<dyn ConfigurableEnvironment>>,
    initializers: Vec<Box<dyn ApplicationContextInitializer>>,
    event_handlers: Vec<Box<dyn ApplicationEventHandler>>,
    default_properties: Option<HashMap<String, AnyValue>>,
    additional_profiles: HashSet<String>,
    application_context_factory: Box<dyn ApplicationContextFactory>,
    application_startup: Box<dyn ApplicationStartup>,
    properties: ApplicationProperties,
    shutdown_hook: Option<ApplicationShutdownHook>,
    application: T,
}

impl<T> NextWebApplication<T>
where
    T: Application,
    T: Default,
{
    /// Creates a new `NextWebApplication` with the given resource loader.
    pub fn new(resource_loader: Option<Arc<dyn ResourceLoader>>) -> Self {
        Self {
            resource_loader,
            add_command_line_properties: true,
            banner: None,
            environment: None,
            initializers: Vec::new(),
            event_handlers: vec![
                Box::new(LoggingEventHandler::default()),
                Box::new(EnvironmentPostProcessorEventHandler::default()),
            ],
            default_properties: None,
            additional_profiles: HashSet::new(),
            properties: Default::default(),
            application_context_factory: Box::new(DefaultApplicationContextFactory::default()),
            application_startup: Box::new(DefaultApplicationStartup::default()),
            shutdown_hook: None,
            application: T::default(),
        }
    }

    /// Run the application.
    pub async fn run(mut self) {
        use futures::FutureExt;

        // The startup future borrows `self`; it is scoped inside this block
        // so the borrow ends before the error handlers consume `self`.
        let future = async {
            let mut startup = StandardStartup::default();
            if self.properties.is_register_shutdown_hook() {
                self.shutdown_hook = Some(ApplicationShutdownHook::default());
            }
            self.send_event(|handlers| handlers.starting(std::any::type_name::<T>()));

            // Start
            let application_arguments = DefaultApplicationArguments::new(std::env::args());
            let environment = self.prepare_environment(&application_arguments);
            let _ = self.print_banner(environment.as_ref());
            let mut context = self.create_application_context()?;
            let application_startup = std::mem::replace(
                &mut self.application_startup,
                Box::new(DefaultApplicationStartup::default()),
            );
            context.set_application_startup(application_startup);
            self.prepare_context(
                context.as_mut(),
                environment.clone(),
                &application_arguments,
            )?;
            self.refresh_context(context.as_mut())?;
            self.after_refresh(context.as_mut(), &application_arguments);
            let time_taken_to_started = startup.started();
            if self.properties.is_log_startup_info() {
                StartupInfoLogger::new(
                    Some(type_name_of_val(&self.application)),
                    environment.as_ref(),
                )
                .log_started(&startup);
            }
            self.send_event(|handlers| handlers.started(context.as_mut(), time_taken_to_started));
            self.call_runners(context.as_mut(), &application_arguments)
                .await?;
            self.send_event(|handlers| handlers.ready(context.as_mut(), startup.ready()));

            self.run_web_server(environment.as_ref(), context.as_mut())
                .await?;
            ApplicationResult::<()>::Ok(())
        };

        match AssertUnwindSafe(future).catch_unwind().await {
            Ok(Err(error)) => self.handle_run_error(error.as_ref()).await,
            Err(payload) => self.handle_run_panic(payload.as_ref()).await,
            Ok(_) => return,
        }
    }

    /// Binds the web server to the specified environment and context.
    async fn run_web_server(
        &mut self,
        environment: &dyn ConfigurableEnvironment,
        ctx: &mut dyn ConfigurableApplicationContext,
    ) -> ApplicationResult<()> {
        let port = environment
            .get_property_or_default("next.server.port", "8080")
            .parse::<u16>()
            .unwrap_or(APPLICATION_DEFAULT_PORT);
        let address = environment.get_property_or_default("next.server.address", "0.0.0.0");
        let socket_addr = format!("{}:{}", address, port).parse::<std::net::SocketAddr>()?;
        let router = self
            .application
            .router(ctx)
            .fallback(|| async { T::fallback() })
            // Prevent program panic caused by users not setting routes
            .route("/_20250101", axum::routing::get(|| async { "a new year!" }));

        let mut web_server = WebServer::new(socket_addr, router);
        web_server.run().await?;

        Ok(())
    }

    fn prepare_environment(
        &mut self,
        application_arguments: &dyn ApplicationArguments,
    ) -> Arc<dyn ConfigurableEnvironment> {
        // Create and configure the environment
        let mut environment = self.get_or_create_environment();
        self.configure_environment(environment.as_mut(), application_arguments.source_args());
        ConfigurationPropertySources::attach(environment.as_mut());

        let additional_profiles = self
            .additional_profiles()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let resource_loader = self.resource_loader.clone();
        self.send_event(|handlers| {
            handlers.environment_prepared(
                environment.as_mut(),
                resource_loader,
                additional_profiles,
            )
        });

        ApplicationInfoPropertySource::move_to_end(environment.as_mut());
        DefaultPropertiesPropertySource::move_to_end(environment.as_mut());
        self.bind_to_application(environment.as_ref());
        if self.environment.is_none() {
            environment = Box::new(ApplicationEnvironment::default());
        }
        ConfigurationPropertySources::attach(environment.as_mut());

        Arc::from(environment)
    }

    fn prepare_context(
        &mut self,
        context: &mut dyn ConfigurableApplicationContext,
        environment: Arc<dyn ConfigurableEnvironment>,
        application_arguments: &DefaultApplicationArguments,
    ) -> ApplicationResult<()> {
        context.set_environment(environment);

        context
            .singleton_factory()?
            .set_allow_override(self.properties.is_allow_override());
        self.apply_initializers(context);
        self.send_event(|handlers| handlers.context_prepared(context));
        if self.properties.is_log_startup_info() {
            self.log_startup_info(context);
            self.log_startup_profile_info(context);
        }

        context
            .singleton_factory()?
            .registry_mut()
            .register_singleton(
                "nextWebApplicationArguments",
                application_arguments.to_owned(),
            );

        self.send_event(|handlers| handlers.context_loaded(context));
        Ok(())
    }

    fn refresh_context(
        &self,
        application_context: &mut dyn ConfigurableApplicationContext,
    ) -> ApplicationResult<()> {
        // Refresh the underlying ApplicationContext.
        application_context.refresh().map_err(Into::into)
    }

    fn send_event<'a, F>(&'a mut self, handler_action: F)
    where
        F: FnOnce(&mut EventHandlers<'a>),
    {
        let mut event_handlers = EventHandlers {
            handlers: &mut self.event_handlers,
            application_startup: self.application_startup.as_mut(),
        };
        handler_action(&mut event_handlers);
    }

    fn get_or_create_environment(&mut self) -> Box<dyn ConfigurableEnvironment> {
        if let Some(environment) = self.environment.take() {
            return environment;
        }

        match self.application_context_factory.create_environment() {
            Some(environment) => return environment,
            None => {
                return DefaultApplicationContextFactory::default()
                    .create_environment()
                    .unwrap_or(Box::new(ApplicationEnvironment::default()))
            }
        }
    }

    /// Template method delegating to configurePropertySources(ConfigurableEnvironment, String[])
    /// and configureProfiles(ConfigurableEnvironment, String[]) in that order. Override this method for complete
    /// control over Environment customization, or one of the above for fine-grained control
    /// over property sources or profiles, respectively.
    fn configure_environment(
        &self,
        environment: &mut dyn ConfigurableEnvironment,
        args: &[String],
    ) {
        self.configure_property_sources(environment, args);
        self.configure_profiles(environment, args);
    }

    /// Add, remove or re-order any PropertySources in this application's environment.
    fn configure_property_sources(
        &self,
        environment: &mut dyn ConfigurableEnvironment,
        args: &[String],
    ) {
        let sources = environment.property_sources();

        if let Some(default_properties) = self.default_properties.as_ref() {
            if !default_properties.is_empty() {
                let default_properties = default_properties
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_string()))
                    .collect();

                DefaultPropertiesPropertySource::add_or_merge(default_properties, sources);
            }
        }

        if self.add_command_line_properties && !args.is_empty() {
            configure_command_line_property_source(sources, args);
        }

        environment
            .property_sources()
            .add_last(Box::new(ApplicationInfoPropertySource::default()));
    }

    /// Configure which profiles are active (or active by default) for this application environment.
    ///  Additional profiles may be activated during configuration file processing through the next.profiles.active property.
    #[allow(unused_variables)]
    fn configure_profiles(&self, environment: &dyn ConfigurableEnvironment, args: &[String]) {}

    /// Bind the environment to the ApplicationProperties.
    fn bind_to_application(&mut self, environment: &dyn ConfigurableEnvironment) {
        self.properties.bind(environment);
    }

    fn print_banner<'a>(
        &'a self,
        environment: &dyn ConfigurableEnvironment,
    ) -> Option<PrintedBanner<'a>> {
        if self.properties.get_banner_mode() == BannerMode::Off {
            return None;
        }

        // The banner borrows the loader it was resolved from, so the default
        // loader is a static rather than a local of this call.
        let resource_loader: &'a dyn ResourceLoader = match self.resource_loader.as_deref() {
            Some(loader) => loader,
            None => &*DEFAULT_RESOURCE_LOADER,
        };

        let banner_printer = ApplicationBannerPrinter::new(resource_loader, self.banner.as_deref());
        if self.properties.get_banner_mode() == BannerMode::Log {
            return Some(banner_printer.print_to_logger(environment));
        }

        Some(
            banner_printer
                .print_to_writer(environment, &mut std::io::stdout())
                .expect("Failed to print banner to stdout"),
        )
    }

    /// Strategy method used to create the ApplicationContext.
    fn create_application_context(
        &self,
    ) -> ApplicationResult<Box<dyn ConfigurableApplicationContext>> {
        match self.application_context_factory.create() {
            Some(context) => Ok(context),
            None => Err("ApplicationContextFactory created none context".into()),
        }
    }

    /// Apply any ApplicationContextInitializers to the context before it is refreshed.
    fn apply_initializers(&mut self, context: &mut dyn ConfigurableApplicationContext) {
        for initializer in self.initializers() {
            initializer.initialize(context);
        }
    }

    /// Called to log startup information, subclasses may override to add additional logging.
    fn log_startup_info(&self, context: &dyn ConfigurableApplicationContext) {
        if context.parent().is_none() {
            StartupInfoLogger::new(
                Some(type_name_of_val(&self.application)),
                context.environment(),
            )
            .log_starting();
        }
    }

    /// Called to log active profile information.
    fn log_startup_profile_info(&self, context: &dyn ConfigurableApplicationContext) {
        if enabled!(Level::INFO) {
            let env = context.environment();
            let active_profiles = Self::quote_profiles(env.active_profiles());
            if active_profiles.is_empty() {
                let default_profiles = Self::quote_profiles(env.default_profiles());
                let message = format!(
                    "{} default {}: ",
                    default_profiles.len(),
                    if default_profiles.len() <= 1 {
                        "profile"
                    } else {
                        "profiles"
                    }
                );
                tracing::info!(
                    "No active profile set, falling back to {}{}",
                    message,
                    default_profiles.join(", ")
                );
            } else {
                let message = if active_profiles.len() == 1 {
                    "1 profile is active: ".to_owned()
                } else {
                    format!("{} profiles are active: ", active_profiles.len())
                };

                tracing::info!("The following {}{}", message, active_profiles.join(", "));
            }
        }
    }

    fn quote_profiles(profiles: &[String]) -> Vec<String> {
        profiles
            .iter()
            .map(|profile| format!("\"{profile}\""))
            .collect()
    }

    /// Called after the context has been refreshed.
    fn after_refresh(
        &self,
        _context: &mut dyn ConfigurableApplicationContext,
        _application_arguments: &dyn ApplicationArguments,
    ) {
    }

    async fn call_runners<'a>(
        &'a self,
        context: &'a mut dyn ConfigurableApplicationContext,
        args: &'a dyn ApplicationArguments,
    ) -> ApplicationResult<()> {
        let singleton_factory = context.singleton_factory()?;
        let mut runners =
            singleton_factory.get_singletons_mut_of_type::<Box<dyn ApplicationRunner>>();
        runners.sort_by_key(|runner| runner.order());

        for runner in runners {
            runner.run(args).await?;
        }

        Ok(())
    }

    /// Converts a caught panic payload into a referenceable error and delegates
    /// to [`Self::handle_run_error`].
    async fn handle_run_panic(self, payload: &(dyn Any + Send)) {
        let error = PanicError::from_payload(payload);
        self.handle_run_error(&error).await;
    }

    fn get_error_reporters(&self) -> Vec<Box<dyn NextWebErrorReporter>> {
        vec![Box::new(ErrorAnalyzers::with_deault_analyzers())]
    }

    /// Handles a error that occurred while running the application.
    ///
    /// The error is passed by reference so it can be inspected by the
    /// registered [`NextWebErrorReporter`]s (for example to locate a cause in
    /// its `source` chain) before the process exits.
    async fn handle_run_error(mut self, error: &(dyn Error + 'static)) {
        self.send_event(|handlers| handlers.error(error));
        let mut reporters = self.get_error_reporters();
        self.report_error(&mut reporters, error);

        if let Some(shutdown_hook) = self.shutdown_hook {
            shutdown_hook.shutdown().await;
        }

        // Exit the process with a non-zero status code to indicate failure.
        std::process::exit(1)
    }

    fn report_error(
        &self,
        error_reporters: &mut [Box<dyn NextWebErrorReporter>],
        error: &(dyn std::error::Error + 'static),
    ) {
        for reporter in error_reporters {
            if reporter.report_error(error) {
                return;
            }
        }

        if enabled!(Level::ERROR) {
            tracing::error!(error = ?error, "Application run failed");
        }
    }

    /// Sets the Banner instance which will be used to print the banner when no static banner file is provided.
    pub fn set_banner<B>(&mut self, banner: B)
    where
        B: Banner + 'static,
    {
        self.banner = Some(Box::new(banner));
    }

    /// SSets the mode used to display the banner when the application runs.
    pub fn set_banner_mode(&mut self, banner_mode: BannerMode) {
        self.properties.set_banner_mode(banner_mode);
    }

    /// Sets if the application information should be logged when the application starts. Defaults to true.
    pub fn set_log_startup_info(&mut self, log_startup_info: bool) {
        self.properties.set_log_startup_info(log_startup_info);
    }

    /// Sets whether properties can be overridden by environment variables.
    pub fn set_allow_override(&mut self, allow_override: bool) {
        self.properties.set_allow_override(allow_override);
    }

    /// Sets if a CommandLinePropertySource should be added to the application context in order to expose arguments. Defaults to true.
    pub fn set_add_command_line_properties(&mut self, add_command_line_properties: bool) {
        self.add_command_line_properties = add_command_line_properties;
    }

    /// Set default environment properties which will be used in addition to those in the existing Environment.
    pub fn set_default_properties(&mut self, default_properties: HashMap<String, AnyValue>) {
        self.default_properties = Some(default_properties);
    }

    /// Set additional profile values to use (on top of those set in system or command line properties).
    pub fn set_additional_profiles<I, V>(&mut self, additional_profiles: I)
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        self.additional_profiles = additional_profiles.into_iter().map(Into::into).collect();
    }

    /// Returns the additional profiles set on this application.
    pub fn additional_profiles(&self) -> &HashSet<String> {
        &self.additional_profiles
    }

    /// Sets the underlying environment that should be used with the created application context.
    pub fn set_environment<E>(&mut self, environment: Option<E>)
    where
        E: ConfigurableEnvironment,
        E: 'static,
    {
        self.environment = environment
            .map(|environment| Box::new(environment) as Box<dyn ConfigurableEnvironment>);
    }

    /// Returns the sources of the application properties.
    pub fn sources(&self) -> &HashSet<String> {
        &self.properties.get_sources()
    }

    /// Sets the sources of the application properties.
    pub fn set_sources(&mut self, sources: HashSet<String>) {
        self.properties.set_sources(sources);
    }

    /// Returns the shutdown hook used to coordinate application shutdown.
    ///
    /// Register callbacks with [`ApplicationShutdownHook::add_hook`] before
    /// running the application and trigger them with
    /// [`ApplicationShutdownHook::shutdown`] when it is time to stop.
    pub fn shutdown_hook(&self) -> Option<&ApplicationShutdownHook> {
        self.shutdown_hook.as_ref()
    }

    /// The ResourceLoader that will be used in the ApplicationContext.
    pub fn resource_loader(&self) -> Option<&Arc<dyn ResourceLoader>> {
        self.resource_loader.as_ref()
    }

    /// Sets the ResourceLoader that will be used in the ApplicationContext.
    pub fn set_resource_loader<R>(&mut self, resource_loader: R)
    where
        R: ResourceLoader,
        R: 'static,
    {
        self.resource_loader = Some(Arc::new(resource_loader));
    }

    /// Sets the application factory that will be used to create the application.
    pub fn set_application_factory<F>(&mut self, application_factory: F)
    where
        F: ApplicationContextFactory,
        F: 'static,
    {
        self.application_context_factory = Box::new(application_factory);
    }

    /// Sets the initializers that will be used to initialize the application context.
    pub fn set_initializers<I>(&mut self, initializers: I)
    where
        I: IntoIterator<Item = Box<dyn ApplicationContextInitializer>>,
    {
        self.initializers = initializers.into_iter().collect();
    }

    /// Adds an initializer to the list of initializers that will be used to initialize the application context.
    pub fn add_initializers<I>(&mut self, initializers: I)
    where
        I: IntoIterator<Item = Box<dyn ApplicationContextInitializer>>,
    {
        self.initializers.extend(initializers.into_iter());
    }

    /// Returns the initializers that will be used to initialize the application context.
    pub fn initializers(&mut self) -> &mut [Box<dyn ApplicationContextInitializer>] {
        &mut self.initializers
    }

    /// Sets the event handlers that will be used to handle application events.
    pub fn set_event_handlers<I>(&mut self, event_handlers: I)
    where
        I: IntoIterator<Item = Box<dyn ApplicationEventHandler>>,
    {
        self.event_handlers = event_handlers.into_iter().collect();
    }

    /// Adds an event handler to the list of event handlers that will be used to handle application events.
    pub fn add_event_handlers<I>(&mut self, event_handlers: I)
    where
        I: IntoIterator<Item = Box<dyn ApplicationEventHandler>>,
    {
        self.event_handlers.extend(event_handlers.into_iter());
    }

    /// Returns the event handlers that will be used to handle application events.
    pub fn event_handlers(&mut self) -> &mut [Box<dyn ApplicationEventHandler>] {
        &mut self.event_handlers
    }

    /// Sets the application startup implementation.
    pub fn set_application_startup<S>(&mut self, startup: S)
    where
        S: ApplicationStartup,
        S: 'static,
    {
        self.application_startup = Box::new(startup);
    }

    /// Returns the application startup implementation, if one has been set.
    pub fn application_startup(&self) -> &dyn ApplicationStartup {
        self.application_startup.as_ref()
    }

    /// Returns the application.
    pub fn application(&self) -> &T {
        &self.application
    }
}

impl<T> Default for NextWebApplication<T>
where
    T: Default,
    T: Application,
{
    fn default() -> Self {
        Self::new(None)
    }
}

/// Tracks application startup timing.
///
/// Mirrors the original `StandardStartup` implementation: it captures a
/// wall-clock start time at construction and reports how long it took to reach
/// the "started" state.
pub struct StandardStartup {
    /// The wall-clock start time, in milliseconds since the Unix epoch.
    start_time_millis: u64,
    /// The monotonic instant captured at construction time.
    start_instant: Instant,
    /// The duration taken to reach the "started" state, once recorded.
    time_taken_to_started: Option<Duration>,
}

impl StandardStartup {
    /// The action label reported by this startup tracker.
    const STANDARD_ACTION: &str = "Started";

    /// Returns the wall-clock start time, in milliseconds since the Unix epoch.
    ///
    /// # Returns
    ///
    /// The start time in milliseconds.
    pub fn start_time_millis(&self) -> u64 {
        self.start_time_millis
    }

    /// Returns the process uptime, if it can be determined.
    ///
    /// There is no portable equivalent of the JVM runtime MX bean, so this
    /// approximates uptime using the elapsed time since construction. A real
    /// implementation could read `/proc/self/stat` on Linux.
    ///
    /// # Returns
    ///
    /// The process uptime in milliseconds, or `None` when it cannot be read.
    pub fn process_uptime_millis(&self) -> Option<u64> {
        match self.start_instant.elapsed() {
            elapsed => Some(elapsed.as_millis() as u64),
        }
    }

    /// Returns the process uptime, if it can be determined.
    ///
    /// There is no portable equivalent of the JVM runtime MX bean, so this
    /// approximates uptime using the elapsed time since construction. A real
    /// implementation could read `/proc/self/stat` on Linux.
    ///
    /// # Returns
    ///
    /// The process uptime, or `None` when it cannot be read.
    pub fn process_uptime(&self) -> Option<Duration> {
        match self.start_instant.elapsed() {
            elapsed => Some(elapsed),
        }
    }

    /// Returns the action label reported by this startup tracker.
    ///
    /// # Returns
    ///
    /// The action label.
    pub fn action(&self) -> &'static str {
        Self::STANDARD_ACTION
    }

    /// Records the time taken to reach the "started" state and returns it.
    ///
    /// Calling this method more than once overwrites the previously recorded
    /// duration.
    ///
    /// # Returns
    ///
    /// The duration taken to start.
    pub fn started(&mut self) -> Duration {
        let elapsed = self.start_instant.elapsed();
        self.time_taken_to_started = Some(elapsed);
        elapsed
    }

    /// Returns the recorded time taken to start.
    ///
    /// # Returns
    ///
    /// The duration taken to start.
    ///
    /// # Errors
    ///
    /// Returns [`StartupError::NotStarted`] when [`Self::started`] has not been
    /// called yet.
    pub fn time_taken_to_started(&self) -> Result<Duration, &'static str> {
        self.time_taken_to_started.ok_or("Not started")
    }

    /// Returns the time taken since the startup tracker was created.
    ///
    /// This mirrors the original private `ready()` helper.
    ///
    /// # Returns
    ///
    /// The elapsed duration.
    pub fn ready(&self) -> Duration {
        self.start_instant.elapsed()
    }
}

impl Default for StandardStartup {
    fn default() -> Self {
        Self {
            start_time_millis: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            start_instant: Instant::now(),
            time_taken_to_started: None,
        }
    }
}

/// Error adapter that turns a caught panic payload into a standard error.
///
/// [`std::panic::catch_unwind`] yields `Box<dyn Any + Send>`, which cannot be
/// handed to a [`NextWebErrorReporter`]. This wrapper extracts the usual panic
/// message (`&str` or `String`) and exposes it as a referenceable
/// [`std::error::Error`].
#[derive(Debug)]
struct PanicError {
    message: String,
}

impl PanicError {
    /// Builds a [`PanicError`] from a panic payload.
    fn from_payload(payload: &(dyn Any + Send)) -> Self {
        let message = payload
            .downcast_ref::<&str>()
            .map(|message| (*message).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "Unknown panic".to_string());

        Self { message }
    }
}

impl std::fmt::Display for PanicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for PanicError {}

struct EventHandlers<'a> {
    handlers: &'a mut [Box<dyn ApplicationEventHandler>],
    application_startup: &'a mut dyn ApplicationStartup,
}

impl<'a> EventHandlers<'a> {
    pub fn starting(&mut self, main_application: &'static str) {
        self.do_with_listeners(
            "next.web.application.starting",
            |handler| {
                handler.handle_event(Event::Starting { args: &[] });
            },
            Some(&mut |step| {
                step.tag("mainApplicationTypeName", main_application.to_owned());
            }),
        );
    }

    pub fn environment_prepared(
        &mut self,
        environment: &'a mut dyn ConfigurableEnvironment,
        resource_loader: Option<Arc<dyn ResourceLoader>>,
        additional_profiles: Vec<String>,
    ) {
        self.do_with_listeners(
            "next.web.application.environment-prepared",
            |handler| {
                handler.handle_event(Event::EnvironmentPrepared(
                    crate::EnvironmentPreparedPayload {
                        args: &[],
                        environment,
                        resource_loader: resource_loader.to_owned(),
                        additional_profiles: additional_profiles.to_owned(),
                    },
                ));
            },
            None,
        );
    }

    pub fn context_prepared(&mut self, context: &'a mut dyn ConfigurableApplicationContext) {
        self.do_with_listeners(
            "next.web.application.context-prepared",
            |handler| {
                handler.handle_event(Event::ContextPrepared { args: &[], context });
            },
            None,
        );
    }

    pub fn context_loaded(&mut self, context: &'a mut dyn ConfigurableApplicationContext) {
        self.do_with_listeners(
            "next.web.application.context-loaded",
            |handler| {
                handler.handle_event(Event::ContextLoaded { args: &[], context });
            },
            None,
        );
    }

    pub fn started(
        &mut self,
        context: &'a mut dyn ConfigurableApplicationContext,
        time_taken: Duration,
    ) {
        self.do_with_listeners(
            "next.web.application.started",
            |handler| {
                handler.handle_event(Event::Started {
                    args: &[],
                    context,
                    time_taken: Some(time_taken),
                });
            },
            None,
        );
    }

    pub fn ready(
        &mut self,
        context: &'a mut dyn ConfigurableApplicationContext,
        time_taken: Duration,
    ) {
        self.do_with_listeners(
            "next.web.application.ready",
            |handler| {
                handler.handle_event(Event::Ready {
                    args: &[],
                    context,
                    time_taken: Some(time_taken),
                });
            },
            None,
        );
    }

    pub fn error(&mut self, err: &(dyn Error + 'static)) {
        self.do_with_listeners(
            "next.web.application.failed",
            |handler| {
                handler.handle_event(Event::Error { args: &[], err });
            },
            Some(&mut |step| {
                step.tag("error", type_name_of_val(err).to_owned());
                step.tag("message", err.to_string());
            }),
        );
    }

    fn do_with_listeners<F>(
        &mut self,
        step_name: &str,
        handler_action: F,
        step_action: Option<&mut dyn FnMut(&mut dyn StartupStep)>,
    ) where
        F: FnMut(&mut Box<dyn ApplicationEventHandler>),
    {
        let mut step = self.application_startup.start(step_name);
        self.handlers.iter_mut().for_each(handler_action);

        if let Some(action) = step_action {
            action(step.as_mut());
        }
        step.end();
    }
}

/// Builds the map representation of the given command line arguments.
///
/// The resulting map can be stored in a [`MapPropertySource`] and combined with
/// other map-backed sources, for example through a [`CompositePropertySource`].
fn command_line_properties(args: &[String]) -> IndexMap<String, String> {
    let source = SimpleCommandLinePropertySource::new(args);
    let mut properties = IndexMap::new();

    for name in source.property_names() {
        if let Some(value) = source.property(name) {
            properties.insert(name.to_string(), value);
        }
    }

    if let Some(value) = source.property(DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME) {
        properties.insert(DEFAULT_NON_OPTION_ARGS_PROPERTY_NAME.to_string(), value);
    }

    properties
}

/// Adds the given command line arguments to the property sources, or merges
/// them with an already registered command line property source.
///
/// When a source named [`COMMAND_LINE_PROPERTY_SOURCE_NAME`] already exists, a
/// composite source holding the new arguments first and the existing source
/// second takes its place. The new arguments therefore keep the highest
/// precedence, and the composite keeps the position of the source it replaces.
/// Otherwise, the new source is added with the highest precedence.
///
/// # Arguments
///
/// * `sources` - The property sources to update.
/// * `args` - The raw command line arguments.
fn configure_command_line_property_source(sources: &mut MutablePropertySources, args: &[String]) {
    let name = COMMAND_LINE_PROPERTY_SOURCE_NAME;

    // Remember the source preceding the command line source, so that the
    // composite can be put back at the same position.
    let previous = sources
        .iter()
        .take_while(|source| source.name() != name)
        .last()
        .map(|source| source.name().to_owned());

    match sources.remove(name) {
        Some(existing) => {
            let mut composite = CompositePropertySource::new(name);
            composite.add_property_source(Box::new(MapPropertySource::new(
                "nextWebApplicationCommandLineArgs".to_owned(),
                command_line_properties(args),
            )));
            composite.add_property_source(existing);

            match previous {
                Some(previous) => sources.add_after(&previous, Box::new(composite)),
                None => sources.add_first(Box::new(composite)),
            }
        }
        None => sources.add_first(Box::new(MapPropertySource::new(
            name.to_owned(),
            command_line_properties(args),
        ))),
    }
}

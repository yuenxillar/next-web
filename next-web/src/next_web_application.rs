use std::{
    any::{type_name_of_val, Any},
    collections::{HashMap, HashSet},
    error::Error,
    future::Future,
    panic::AssertUnwindSafe,
    sync::{Arc, LazyLock},
};

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use next_web_context::{ApplicationEvent, ApplicationListener};
use next_web_core::{
    anys::any_value::AnyValue,
    env::ConfigurableEnvironment,
    io::{DefaultResourceLoader, ResourceLoader},
    metrics::{ApplicationStartup, DefaultApplicationStartup},
};
use next_web_singletons::factory::ListableSingletonFactory;
use tracing::{enabled, Level};

use crate::{
    application::Application, application_banner_printer::PrintedBanner, banner::BannerMode,
    context::ApplicationEnvironment, diagnostics::error_analyzers::ErrorAnalyzers,
    ApplicationArguments, ApplicationBannerPrinter, ApplicationContextFactory,
    ApplicationContextInitializer, ApplicationProperties, ApplicationRunner,
    ApplicationShutdownHook, Banner, ConfigurableApplicationContext, DefaultApplicationArguments,
    DefaultApplicationContextFactory, NextWebErrorReporter, StartupInfoLogger,
};

/// Loader used when the application does not configure one of its own.
///
/// It is created once, so that a banner resolved through it stays valid for as
/// long as it is borrowed.
static DEFAULT_RESOURCE_LOADER: LazyLock<DefaultResourceLoader> =
    LazyLock::new(DefaultResourceLoader::default);

type ApplicationResult<T> = Result<T, Box<dyn Error>>;

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

pub struct NextWebApplication<T> {
    add_command_line_properties: bool,
    banner: Option<Box<dyn Banner>>,
    resource_loader: Option<Arc<dyn ResourceLoader>>,
    environment: Option<Box<dyn ConfigurableEnvironment>>,
    initializers: Vec<Box<dyn ApplicationContextInitializer>>,
    listeners: Vec<Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>>,
    default_properties: Option<HashMap<String, AnyValue>>,
    additional_profiles: HashSet<String>,
    application_context_factory: Box<dyn ApplicationContextFactory>,
    application_startup: Option<Box<dyn ApplicationStartup>>,
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
            listeners: Vec::new(),
            default_properties: None,
            additional_profiles: HashSet::new(),
            properties: Default::default(),
            application_context_factory: Box::new(DefaultApplicationContextFactory::default()),
            application_startup: Some(Box::new(DefaultApplicationStartup::default())),
            shutdown_hook: None,
            application: T::default(),
        }
    }

    /// Run the application.
    pub fn run(mut self) -> impl Future<Output = ()> {
        async move {
            use futures::FutureExt;

            if self.properties.is_register_shutdown_hook() {
                self.shutdown_hook = Some(ApplicationShutdownHook::default());
            }

            let application_arguments = DefaultApplicationArguments::new(std::env::args());
            let environment = self.prepare_environment(&application_arguments);
            let _ = self.print_banner(environment.as_ref());
            let mut context = self.create_application_context();
            if let Some(application_startup) = self.application_startup.take() {
                context.set_application_startup(application_startup);
            }

            let result = AssertUnwindSafe(async {
                let mut startup = StandardStartup::default();

                self.prepare_context()?;
                self.refresh_context(context.as_mut())?;
                self.after_refresh(context.as_mut(), &application_arguments);
                let _ = startup.started();
                if self.properties.is_log_startup_info() {
                    StartupInfoLogger::new(
                        Some(type_name_of_val(&self.application)),
                        environment.as_ref(),
                    )
                    .log_started(&startup);
                }
                self.call_runners(context.as_mut(), &application_arguments)
                    .await?;

                ApplicationResult::<()>::Ok(())
            })
            .catch_unwind()
            .await;

            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    self.handle_run_error(context.as_mut(), error.as_ref())
                        .await
                }
                Err(payload) => {
                    self.handle_run_panic(context.as_mut(), payload.as_ref())
                        .await
                }
            }
        }
    }

    fn prepare_environment(
        &mut self,
        application_arguments: &dyn ApplicationArguments,
    ) -> Box<dyn ConfigurableEnvironment> {
        // Create and configure the environment
        let environment = self.get_or_create_environment();
        self.configure_environment(environment.as_ref(), application_arguments.source_args());

        self.bind_to_application(environment.as_ref());

        environment
    }

    fn prepare_context(&mut self) -> ApplicationResult<()> {
        Ok(())
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

    fn refresh_context(
        &self,
        application_context: &mut dyn ConfigurableApplicationContext,
    ) -> ApplicationResult<()> {
        if self.properties.is_register_shutdown_hook() {}

        // Refresh the underlying ApplicationContext.
        application_context.refresh().map_err(Into::into)
    }

    /// Template method delegating to configurePropertySources(ConfigurableEnvironment, String[])
    /// and configureProfiles(ConfigurableEnvironment, String[]) in that order. Override this method for complete
    /// control over Environment customization, or one of the above for fine-grained control
    /// over property sources or profiles, respectively.
    fn configure_environment(&self, environment: &dyn ConfigurableEnvironment, args: &[String]) {
        self.configure_property_sources(environment, args);
        self.configure_profiles(environment, args);
    }

    /// Add, remove or re-order any PropertySources in this application's environment.
    fn configure_property_sources(
        &self,
        environment: &dyn ConfigurableEnvironment,
        args: &[String],
    ) {
    }

    fn configure_profiles(&self, environment: &dyn ConfigurableEnvironment, args: &[String]) {}

    /// Bind the environment to the ApplicationProperties.
    fn bind_to_application(&self, environment: &dyn ConfigurableEnvironment) {}

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

    fn call_runners<'a>(
        &'a self,
        context: &'a mut dyn ConfigurableApplicationContext,
        args: &'a dyn ApplicationArguments,
    ) -> impl Future<Output = ApplicationResult<()>> + 'a {
        async {
            let singleton_factory = context.singleton_factory()?;
            let mut runners =
                singleton_factory.get_singletons_mut_of_type::<Box<dyn ApplicationRunner>>();
            runners.sort_by_key(|runner| runner.order());

            for runner in runners {
                runner.run(args).await?;
            }

            Ok(())
        }
    }

    /// Converts a caught panic payload into a referenceable error and delegates
    /// to [`Self::handle_run_error`].
    async fn handle_run_panic(
        self,
        context: &mut dyn ConfigurableApplicationContext,
        payload: &(dyn Any + Send),
    ) {
        let error = PanicError::from_payload(payload);
        self.handle_run_error(context, &error).await;
    }

    fn get_error_reporters(&self) -> Vec<Box<dyn NextWebErrorReporter>> {
        vec![Box::new(ErrorAnalyzers::with_deault_analyzers())]
    }

    /// Handles a error that occurred while running the application.
    ///
    /// The error is passed by reference so it can be inspected by the
    /// registered [`NextWebErrorReporter`]s (for example to locate a cause in
    /// its `source` chain) before the process exits.
    async fn handle_run_error(
        self,
        _context: &mut dyn ConfigurableApplicationContext,
        error: &(dyn Error + 'static),
    ) {
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

    /// Sets the listeners that will be used to handle application events.
    pub fn set_listeners<I>(&mut self, listeners: I)
    where
        I: IntoIterator<Item = Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>>,
    {
        self.listeners = listeners.into_iter().collect();
    }

    /// Adds a listener to the list of listeners that will be used to handle application events.
    pub fn add_listeners<I>(&mut self, listeners: I)
    where
        I: IntoIterator<Item = Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>>,
    {
        self.listeners.extend(listeners.into_iter());
    }

    /// Returns the listeners that will be used to handle application events.
    pub fn listeners(&self) -> &[Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>] {
        &self.listeners
    }

    /// Sets the application startup implementation.
    pub fn set_application_startup<S>(&mut self, startup: S)
    where
        S: ApplicationStartup,
        S: 'static,
    {
        self.application_startup = Some(Box::new(startup));
    }

    /// Returns the application startup implementation, if one has been set.
    pub fn application_startup(&self) -> Option<&dyn ApplicationStartup> {
        self.application_startup.as_deref()
    }

    /// Returns the application.
    pub fn application(&self) -> &T {
        &self.application
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

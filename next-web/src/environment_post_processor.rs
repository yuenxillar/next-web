use next_web_core::env::ConfigurableEnvironment;

/// Allows for customization of the application's [`Environment`] before the
/// application context is refreshed.
pub trait EnvironmentPostProcessor {
    /// Post-processes the given `environment`.
    ///
    /// # Parameters
    ///
    /// * `environment` - The environment to post-process.
    /// * `application` - The application to which the environment belongs.
    fn post_process_environment(&mut self, environment: &mut dyn ConfigurableEnvironment);
}

/// Provides a blanket implementation of [`EnvironmentPostProcessor`] for
/// closures.
impl<F> EnvironmentPostProcessor for F
where
    F: Fn(&mut dyn ConfigurableEnvironment),
{
    fn post_process_environment(&mut self, environment: &mut dyn ConfigurableEnvironment) {
        self(environment)
    }
}

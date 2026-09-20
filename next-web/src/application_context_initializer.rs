use crate::ConfigurableApplicationContext;

/// Initializes an application context.
pub trait ApplicationContextInitializer {
    /// Initialize the given application context.
    fn initialize(&mut self, application_context: &mut dyn ConfigurableApplicationContext);
}

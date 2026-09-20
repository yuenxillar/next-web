//! Strategy trait for creating the [`ConfigurableApplicationContext`] used by an
//! application.
//!
//! Created contexts should be returned in their default form, with the
//! application responsible for configuring and refreshing the context.

use next_web_core::env::ConfigurableEnvironment;

use crate::ConfigurableApplicationContext;

/// Strategy trait for creating the [`ConfigurableApplicationContext`] used by an
/// application.
///
/// Created contexts should be returned in their default form, with the
/// application responsible for configuring and refreshing the context.
pub trait ApplicationContextFactory: Send + Sync {
    /// Creates a new [`ConfigurableEnvironment`] to be set on the created
    /// application context.
    ///
    /// The result of this method must match the type returned by
    /// [`Self::environment_type`].
    ///
    /// The default implementation returns `None`, meaning the default
    /// environment should be used.
    ///
    /// # Returns
    ///
    /// An environment instance, or `None` to use the default.
    fn create_environment(&self) -> Option<Box<dyn ConfigurableEnvironment>> {
        None
    }

    /// Creates the [`ConfigurableApplicationContext`] for an application.
    ///
    /// # Returns
    ///
    /// The newly created application context, or `None` if no context could be
    /// created.
    fn create(&self) -> Option<Box<dyn ConfigurableApplicationContext>>;
}

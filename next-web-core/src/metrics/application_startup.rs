//! Instruments the application startup phase using [`StartupStep`]s.
//!
//! The core container and its infrastructure components can use the
//! [`ApplicationStartup`] to mark steps during application startup and collect
//! data about the execution context or their processing time.

use crate::metrics::StartupStep;

/// Instruments the application startup phase using [`StartupStep`]s.
///
/// # Purpose
///
/// Implementations mark steps during application startup and collect data about
/// the execution context or their processing time.
///
/// # Implementations
///
/// The default implementation is [`DefaultApplicationStartup`], which records
/// nothing and is designed for minimal overhead. Other implementations may
/// buffer steps, emit events, or write to a profiler.
pub trait ApplicationStartup: Send + Sync {
    /// Creates a new step and marks its beginning.
    ///
    /// A step name describes the current action or phase. This technical name
    /// should be "." namespaced and can be reused to describe other instances
    /// of the same step during application startup.
    ///
    /// # Arguments
    ///
    /// * `name` - The step name.
    ///
    /// # Returns
    ///
    /// A newly started [`StartupStep`].
    fn start(&self, name: &str) -> Box<dyn StartupStep>;
}

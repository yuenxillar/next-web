use std::error::Error;

use next_web_core::{BoxFuture, Ordered};

use crate::ApplicationArguments;

/// A trait for running application logic after the application context has been
/// fully initialized and all beans have been registered.
///
/// Implementors of this trait are typically used to execute startup tasks such
/// as warming up caches, establishing external connections, seeding data, or
/// launching auxiliary services. Runners are invoked in the order determined
/// by their [`Ordered`] implementation, allowing fine-grained control over the
/// startup sequence.
///
/// # Examples
///
/// ```ignore
/// struct MyRunner;
///
/// impl Ordered for MyRunner {
///     fn order(&self) -> i32 {
///         0
///     }
/// }
///
/// impl ApplicationRunner for MyRunner {
///     fn run(
///         &mut self,
///         args: &dyn ApplicationArguments,
///     ) -> BoxFuture<'_, Result<(), Box<dyn Error>>> {
///         Box::pin(async move {
///             // Perform startup logic here.
///             Ok(())
///         })
///     }
/// }
/// ```
pub trait ApplicationRunner
where
    Self: Ordered,
{
    /// Executes the application runner's logic.
    ///
    /// This method is called once during application startup, after the
    /// application context has been prepared. The provided [`ApplicationArguments`]
    /// can be used to access command-line arguments and other runtime
    /// configuration.
    ///
    /// The returned future is boxed to allow trait objects and to keep the
    /// trait object-safe. Any error returned will typically cause the
    /// application startup to abort.
    ///
    /// # Parameters
    ///
    /// * `args` - The application arguments supplied at startup, providing
    ///   access to parsed command-line options and other runtime context.
    ///
    /// # Returns
    ///
    /// A boxed future that resolves to `Ok(())` on success, or an error if the
    /// runner fails to complete its work.
    fn run(&mut self, args: &dyn ApplicationArguments)
        -> BoxFuture<'_, Result<(), Box<dyn Error>>>;
}

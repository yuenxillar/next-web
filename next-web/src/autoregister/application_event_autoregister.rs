use next_web_context::ApplicationContext;
use next_web_core::BoxFuture;

/// A trait for application event handlers that register themselves
/// automatically.
///
/// Implementors are collected at compile time via [`inventory`] and later
/// invoked by the application during startup, allowing event listeners to be
/// added without a central registration list.
///
/// # Example
///
/// ```ignore
/// struct StartupListener;
///
/// impl ApplicationEventAutoRegister for StartupListener {
///     fn register<'life_a>(
///         &'life_a self,
///         ctx: &'life_a mut dyn ApplicationContext,
///     ) -> BoxFuture<'life_a, ()> {
///         Box::pin(async move {
///             // subscribe to events using `ctx`
///         })
///     }
/// }
///
/// submit_application_event_autoregister!(StartupListener);
/// ```
pub trait ApplicationEventAutoRegister
where
    Self: Send + Sync,
    Self: 'static,
{
    /// Registers the event listener with the given application context.
    ///
    /// # Parameters
    ///
    /// - `ctx`: A mutable reference to the [`ApplicationContext`] used to
    ///   subscribe listeners or wire up event handling.
    ///
    /// # Returns
    ///
    /// A boxed future that completes once registration has finished. The
    /// future borrows `self` and `ctx` for `'life_a`, so neither can be moved
    /// or dropped while the future is pending.
    fn register<'life_a>(
        &'life_a self,
        ctx: &'life_a mut dyn ApplicationContext,
    ) -> BoxFuture<'life_a, ()>;
}

// Collects all static references to `ApplicationEventAutoRegister`
// implementations. The `inventory` crate uses this collection to gather every
// handler submitted through `submit_application_event_autoregister!`; the
// application can iterate over
// `inventory::iter::<&'static dyn ApplicationEventAutoRegister>` at startup
// to call each `register` method.
inventory::collect!(&'static dyn ApplicationEventAutoRegister);

// Submits a type implementing `ApplicationEventAutoRegister` to the global
// registry at compile time.
//
// This macro wraps `inventory::submit!`, casting the given type to a
// `&dyn ApplicationEventAutoRegister` trait object. The type is instantiated as
// a static value, so it must be a unit struct or otherwise constructible in a
// const/static context.
//
// # Parameters
//
// - `$ty`: An identifier referring to a type that implements
//   `ApplicationEventAutoRegister`.
//
// # Example
//
// ```ignore
// submit_application_event_autoregister!(StartupListener);
// ```
#[macro_export]
macro_rules! submit_application_event_autoregister {
    ($ty:ident) => {
        ::next_web::macros::submit! {
            &$ty as &dyn ::next_web::autoregister::application_event_autoregister::ApplicationEventAutoRegister
        }
    };
}

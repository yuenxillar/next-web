use axum::Router;

use crate::context::RouterContext;

/// A trait for types that can automatically register themselves as HTTP handlers.
///
/// Implementors of this trait are responsible for adding their routes to an
/// [`axum::Router`] and may use the provided [`RouterContext`] to share state
/// or configuration during registration.
///
/// This trait is typically used together with the [`submit_http_handler!`] macro and
/// the [`inventory`] crate to enable decentralized, compile-time route
/// registration: each handler module submits itself to a global registry, and
/// the application can later iterate over all registered handlers to build the
/// final router.
///
/// # Example
///
/// ```ignore
/// struct UserHandler;
///
/// impl HttpHandlerAutoRegister for UserHandler {
///     fn register<'a>(&self, router: Router, context: &'a mut RouterContext) -> Router {
///         router.route("/users", get(list_users))
///     }
/// }
///
/// submit_http_handler!(UserHandler);
/// ```
pub trait HttpHandlerAutoRegister
where
    Self: Send + Sync + 'static,
{
    /// Registers the handler's routes onto the given router.
    ///
    /// # Parameters
    ///
    /// - `__router`: The [`Router`] to which the handler's routes should be added.
    /// - `__context`: A mutable reference to the shared [`RouterContext`], which
    ///   can be used to read or update application-wide configuration during
    ///   registration.
    ///
    /// # Returns
    ///
    /// The updated [`Router`] with the handler's routes attached.
    fn register<'a>(&self, __router: Router, __context: &'a mut RouterContext) -> Router;
}

// Submits a type implementing `HttpHandlerAutoRegister` to the global
// handler registry.
//
// This macro wraps `inventory::submit!` and registers the handler at compile
// time. Once submitted, it will be discovered automatically when building the
// router.
//
// # Parameters
//
// - `$ty`: A unit struct or const-constructible type implementing
//   `HttpHandlerAutoRegister`.
//
// # Example
//
// ```ignore
// submit_handler!(HealthHandler);
// ```
inventory::collect!(&'static dyn HttpHandlerAutoRegister);

/// Submits a type implementing [`HttpHandlerAutoRegister`] to the global
/// handler registry.
///
/// This macro is a thin wrapper around [`inventory::submit!`] that casts the
/// given type to a `&dyn HttpHandlerAutoRegister` trait object and registers it
/// at compile time. Once submitted, the handler will be discovered automatically
/// when the application builds its router.
///
/// # Parameters
///
/// - `$ty`: An identifier referring to a type that implements
///   [`HttpHandlerAutoRegister`]. The type must have a zero-argument constructor
///   or be a unit struct, since the macro creates a static instance of it.
///
/// # Example
///
/// ```ignore
/// struct HealthHandler;
///
/// impl HttpHandlerAutoRegister for HealthHandler {
///     fn register<'a>(&self, router: Router, _context: &'a mut RouterContext) -> Router {
///         router.route("/health", get(|| async { "ok" }))
///     }
/// }
///
/// submit_http_handler!(HealthHandler);
/// ```
#[macro_export]
macro_rules! submit_http_handler {
    ($ty:ident) => {
        ::next_web::macros::submit! {
            &$ty as &dyn ::next_web::autoregister::http_handler_autoregister::HttpHandlerAutoRegister
        }
    };
}

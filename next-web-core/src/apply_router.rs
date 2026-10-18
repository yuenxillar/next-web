use dyn_clone::DynClone;

use crate::{ApplicationContext, Ordered};

/// A router that can be applied to an [`ApplicationContext`] to build an
/// [`axum::Router`].
///
/// An implementation of this trait contributes a router to the application.
/// The router is applied while the application is being built, and the
/// returned router is combined with the routers of the other implementations.
///
/// The trait is object safe, so implementations are usually stored as trait
/// objects, for example `Box<dyn ApplyRouter>` or `Arc<dyn ApplyRouter>`. The
/// [`DynClone`] supertrait allows such a trait object to be cloned.
///
/// # Examples
///
/// ```ignore
/// #[derive(Clone)]
/// struct HealthRouter;
///
/// impl ApplyRouter for HealthRouter {
///     fn apply(&mut self, _ctx: &mut dyn ApplicationContext) -> axum::Router {
///         axum::Router::new().route("/health", axum::routing::get(|| async { "ok" }))
///     }
/// }
///
/// impl Ordered for HealthRouter {
///     fn order(&self) -> i32 {
///         0
///     }
/// }
/// ```
pub trait ApplyRouter
where
    Self: DynClone,
    Self: Send + Sync,
    Self: Ordered,
{
    /// Applies the router to the given application context and returns the
    /// router the context serves.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The application context the router is applied to. The context
    ///   is passed as a mutable trait object, so an implementation reads the
    ///   state it needs, such as the configured services, without owning the
    ///   context.
    ///
    /// # Returns
    ///
    /// The [`axum::Router`] of this implementation, which the caller combines
    /// with the routers of the other implementations. Returning an empty
    /// router is valid, and the router is then a no-op.
    fn apply(&mut self, ctx: &mut dyn ApplicationContext) -> axum::Router;
}

// Implements `Clone` for `Box<dyn ApplyRouter>` and the other trait objects
// of `ApplyRouter`, so a router can be cloned without knowing its concrete
// type.
//
// The macro forwards `Clone::clone` to the `dyn_clone::clone_box` method of
// the `DynClone` supertrait, which every implementation provides through the
// blanket implementation of `DynClone`.
dyn_clone::clone_trait_object!(ApplyRouter);

//! The application context service provider interface.
//!
//! [`ApplicationContext`] is the seam between the framework and the code that is
//! written against it:
//!
//! - generated code produced by the attribute macros of the framework,
//! - auto configurations and auto registrations written by other libraries.
//!
//! Because of that, the trait lives in this crate: depending on the framework at
//! runtime ([`next_web`](https://docs.rs/next-web)) pulls in a web server, an
//! http client and much more, which would be a heavy requirement for a library
//! that only wants to contribute a few singletons.
//!
//! # Type erasure
//!
//! The trait is deliberately object safe, so that a reference to `dyn
//! ApplicationContext` can be handed to a provider constructor or to an
//! extension point. Its methods therefore operate on type erased instances
//! (`Box<dyn Any + Send + Sync>`) instead of being generic over the type of the
//! instance. The generic, type safe methods that the generated code uses are
//! provided by [`ApplicationContextExt`](crate::ApplicationContextExt).

use std::any::{Any, TypeId};
use std::sync::Arc;

use next_web_singletons::factory::support::Key;

use crate::{ApplicationEventPublisher, MessageSource};

/// The name of the  singleton in the context.
/// If none is supplied, message resolution is delegated to the parent.
pub const MESSAGE_SOURCE_SINGLETON_NAME: &str = "messageSource";

/// The name of the ApplicationEventMulticaster singleton in the context.
/// If none is supplied, a SimpleApplicationEventMulticaster is used.
pub const APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME: &str = "applicationEventMulticaster";

/// The name of the ResourceLoader singleton in the context.
/// If none is supplied, a SimpleResourceLoader is used.
pub const RESOURCE_LOADER_SINGLETON_NAME: &str = "resourceLoader";

/// The name of the ApplicationEnvironment singleton in the context.
/// If none is supplied, a SimpleApplicationEnvironment is used.
pub const APPLICATION_ENVIRONMENT_SINGLETON_NAME: &str = "applicationEnvironment";

/// Signature of the function a context stores next to an instance so that owned
/// copies of it can be produced later.
///
/// The function receives the instance as a type erased reference and returns a
/// boxed copy of it, which lets a context hand out owned values without knowing
/// their concrete type.
pub type InstanceClone =
    Arc<dyn Fn(&(dyn Any + Send + Sync)) -> Box<dyn Any + Send + Sync> + Send + Sync>;

/// Returns the name a singleton of type `T` is registered with by default.
///
/// The name is the last segment of the type name with its first character lower
/// cased, so `MyService` becomes `myService` and
/// `crate::service::MyService` becomes `myService` as well.
///
/// # Type Parameters
///
/// * `T` - The type whose default name is returned.
pub fn default_singleton_name<T: ?Sized>() -> String {
    let type_name = std::any::type_name::<T>();
    let name = type_name.rsplit("::").next().unwrap_or_default();

    let mut characters = name.chars();
    match characters.next() {
        Some(first) => {
            let mut default_name = String::with_capacity(name.len());
            default_name.extend(first.to_lowercase());
            default_name.push_str(characters.as_str());
            default_name
        }
        None => name.to_string(),
    }
}

/// The application context service provider interface.
///
/// A context owns the singletons of the application and is the entry point used
/// to obtain them. Besides the client methods ([`Self::id`],
/// [`Self::application_name`] and [`Self::startup_date`]), it exposes the
/// low level, type erased operations a dependency injection container has to
/// provide. Those operations are expressed in terms of [`Key`], which pairs the
/// type of an instance with the name it is registered under.
///
/// The accessors that work on type erased instances end with `_boxed`, which
/// keeps them apart from the type safe methods of
/// [`ApplicationContextExt`](crate::ApplicationContextExt): the context has a
/// single concept of a singleton, reachable either through a [`Key`] here or
/// through a type parameter there.
///
/// Implementations are expected to be usable as `&mut dyn ApplicationContext`,
/// which is why every method of this trait is object safe.
pub trait ApplicationContext
where
    Self: Send + Sync,
    Self: ApplicationEventPublisher,
    Self: MessageSource,
{
    /// Return the unique id of this application context.
    fn id(&self) -> &str;

    /// Return a name for the deployed application that this context belongs to.
    fn application_name(&self) -> &str;

    /// Return the timestamp when this context was first loaded.
    fn startup_date(&self) -> i64;

    /// Stores the given instance under the given key.
    ///
    /// The `clone` function is `Some` when the context should be able to hand
    /// out owned copies of the instance, and `None` when only borrows of it may
    /// be produced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    /// * `instance` - The type erased instance to store.
    /// * `clone` - The function producing owned copies of the instance.
    fn insert_singleton_boxed(
        &mut self,
        key: Key,
        instance: Box<dyn Any + Send + Sync>,
        clone: Option<InstanceClone>,
    );

    /// Returns whether an instance is registered under the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    fn contains_singleton_boxed(&self, key: &Key) -> bool;

    /// Returns a shared, type erased reference to the instance registered under
    /// the given key, or `None` when there is none.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    fn get_singleton_boxed(&self, key: &Key) -> Option<&(dyn Any + Send + Sync)>;

    /// Returns a mutable, type erased reference to the instance registered under
    /// the given key, or `None` when there is none.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    fn get_singleton_boxed_mut(&mut self, key: &Key) -> Option<&mut (dyn Any + Send + Sync)>;

    /// Removes the instance registered under the given key and returns it.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    fn remove_singleton_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>>;

    /// Produces an owned, type erased instance for the given key.
    ///
    /// The context returns a stored instance when it has one, and constructs the
    /// instance through a registered provider otherwise. Returns `None` when the
    /// key is unknown to the context.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the instance.
    fn resolve_boxed(&mut self, key: &Key) -> Option<Box<dyn Any + Send + Sync>>;

    /// Produces owned, type erased instances for every key whose instance has
    /// the given type.
    ///
    /// This is the operation behind the `resolve_by_type` client method, which
    /// has to collect instances of a type without knowing their names.
    ///
    /// # Arguments
    ///
    /// * `ty` - The [`TypeId`] of the instances to produce.
    fn resolve_all_of_type(&mut self, ty: TypeId) -> Vec<Box<dyn Any + Send + Sync>>;

    /// Returns the keys of the instances whose type has the given [`TypeId`].
    ///
    /// The keys cover both the instances the context holds and the instances
    /// its providers can still create, so a key that is returned does not have
    /// to be resolvable yet. This is the operation behind the type wide
    /// operations of [`ApplicationContextExt`](crate::ApplicationContextExt).
    ///
    /// # Arguments
    ///
    /// * `ty` - The [`TypeId`] of the instances to look for.
    fn keys_of_type(&self, ty: TypeId) -> Vec<Key>;
}

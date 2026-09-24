use std::error::Error;

use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use crate::{ApplicationContext, Ordered};

/// An auto-configuration that contributes beans to the application context.
///
/// Implementors describe a self-contained unit of configuration (analogous to
/// Spring Boot's `@AutoConfiguration` classes). Each auto-configuration is
/// applied to the [`ApplicationContext`] in ascending `order()` during startup.
///
/// Implementations must be `Send + Sync + 'static` so they can be stored and
/// driven across threads, and `DynClone` so the registry can clone them into
/// boxed trait objects.
#[async_trait]
pub trait AutoConfiguration
where
    Self: Send + Sync,
    Self: 'static,
    Self: Ordered,
    Self: DynClone,
{
    /// Applies this auto-configuration to the given application context.
    ///
    /// Typically used to register beans, properties, or other resources on
    /// `ctx`. Implementations should be idempotent where practical and must
    /// return an error if the configuration cannot be applied.
    ///
    /// # Errors
    ///
    /// Returns an error if any part of the configuration fails, which will
    /// abort the startup sequence.
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>>;

    /// Determines whether the condition matches.
    ///
    /// # Parameters
    /// - `context`: the context providing access to the environment,
    ///   component registry, and resource loader.
    ///
    /// # Returns
    /// `true` if the condition matches and the component may be registered,
    /// or `false` to veto registration of the annotated component.
    #[allow(unused_variables)]
    fn matches(&self, ctx: &dyn ApplicationContext) -> bool {
        true
    }
}

clone_trait_object!(AutoConfiguration where Self: Send + Sync);

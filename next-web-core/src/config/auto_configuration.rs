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
///
/// # Ordering
///
/// `order()` is what the configurations that do not constrain each other are
/// applied in. A configuration that has to hold a place relative to another one
/// declares it with [`Self::auto_configure_before`] or
/// [`Self::auto_configure_after`], which name the configurations by
/// [`Self::auto_configuration_name`]:
///
/// - a configuration declared with `#[auto_configuration(before = ["X"])]` is
///   applied before `X`, even when its `order()` is the higher one,
/// - a name no configuration of the application provides is ignored, so a
///   starter can name a configuration that is not part of the application,
/// - the configurations that name each other have to be orderable, which is
///   what makes a cycle among them an error that stops the startup.
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

    /// Returns the name the `before` and `after` lists of the other
    /// configurations refer to this configuration with.
    ///
    /// The name defaults to the name of the type of the configuration, so a
    /// configuration named `RedisAutoConfiguration` is referenced with
    /// `before = ["RedisAutoConfiguration"]`. The name is the last segment of
    /// the name of the type without the arguments of a generic type, so it does
    /// not depend on the module or the crate a configuration is declared in.
    ///
    /// A configuration that is declared with `#[auto_configuration]` is named
    /// after the annotated type, and not after the type the macro generates.
    fn auto_configuration_name(&self) -> &'static str {
        configuration_name::<Self>()
    }

    /// Returns the names of the configurations this one is applied before.
    ///
    /// The constraint wins over the order [`Ordered::order`] declares, which is
    /// what lets a configuration of an application take its place among the
    /// configurations of the starters it depends on. A name no configuration of
    /// the application provides is ignored.
    fn auto_configure_before(&self) -> &'static [&'static str] {
        &[]
    }

    /// Returns the names of the configurations this one is applied after.
    ///
    /// This is the counterpart of [`Self::auto_configure_before`].
    fn auto_configure_after(&self) -> &'static [&'static str] {
        &[]
    }
}

clone_trait_object!(AutoConfiguration where Self: Send + Sync);

/// Returns the name a configuration of the type `T` is referenced with.
///
/// The name is the last segment of the name of the type without the arguments
/// of a generic type, so `crate::redis::RedisAutoConfiguration` is named
/// `RedisAutoConfiguration`, and `crate::config::MyConfiguration<MyProperties>`
/// is named `MyConfiguration`.
pub(crate) fn configuration_name<T: ?Sized>() -> &'static str {
    let name = std::any::type_name::<T>();

    // The arguments of a generic type are named with the separators of a path
    // as well, so they are cut off before the last segment is taken.
    let name = name.split('<').next().unwrap_or(name);

    name.rsplit("::").next().unwrap_or(name)
}

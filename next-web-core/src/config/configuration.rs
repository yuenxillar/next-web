use std::error::Error;

use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use super::auto_configuration::configuration_name;
use crate::{ApplicationContext, Ordered};

/// A configuration of an application.
///
/// A configuration is a self-contained unit that contributes beans to the
/// application context while the application starts. It is the counterpart of
/// the [`AutoConfiguration`](super::auto_configuration::AutoConfiguration) of a
/// starter, and it is the abstraction a `#[configuration]` class implements.
///
/// The configurations of an application are applied **before** the
/// auto-configurations, so that a configuration can prepare the state an
/// auto-configuration of a starter depends on. The configurations are applied
/// in ascending [`Ordered::order`], refined by the `before` and `after` names
/// they declare, which is the same ordering the auto-configurations use.
///
/// Implementations must be `Send + Sync + 'static` so they can be stored and
/// driven across threads, and `DynClone` so the registry can clone them into
/// boxed trait objects.
#[async_trait]
pub trait Configuration
where
    Self: Send + Sync,
    Self: 'static,
    Self: Ordered,
    Self: DynClone,
{
    /// Applies this configuration to the given application context.
    ///
    /// Typically used to register beans, properties, or other resources on
    /// `ctx`. Implementations should be idempotent where practical and must
    /// return an error if the configuration cannot be applied.
    ///
    /// # Errors
    ///
    /// Returns an error if any part of the configuration fails, which aborts the
    /// startup sequence.
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>>;

    /// Determines whether the configuration is applied.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the configuration is applied to.
    ///
    /// # Returns
    ///
    /// `true` if the configuration is applied, or `false` to skip it.
    #[allow(unused_variables)]
    fn matches(&self, ctx: &dyn ApplicationContext) -> bool {
        true
    }

    /// Returns the name the `before` and `after` lists of the other
    /// configurations refer to this one with.
    ///
    /// The name defaults to the name of the type of the configuration, which is
    /// the last segment of the name of the type without the arguments of a
    /// generic type.
    fn configuration_name(&self) -> &'static str {
        configuration_name::<Self>()
    }

    /// Returns the names of the configurations this one is applied before.
    ///
    /// The constraint wins over the order [`Ordered::order`] declares, which is
    /// what lets a configuration of an application take its place among the
    /// configurations it depends on. A name no configuration of the application
    /// provides is ignored.
    fn configure_before(&self) -> &'static [&'static str] {
        &[]
    }

    /// Returns the names of the configurations this one is applied after.
    ///
    /// This is the counterpart of [`Self::configure_before`].
    fn configure_after(&self) -> &'static [&'static str] {
        &[]
    }
}

clone_trait_object!(Configuration where Self: Send + Sync);

use dyn_clone::DynClone;

use std::error::Error;

use crate::env::{BindError, ConfigurableEnvironment};

/// Configuration properties of an application.
///
/// The properties a type declares with `#[configuration_properties(prefix =
/// "next.messages")]` are bound from the [`ConfigurableEnvironment`] the
/// application runs in, so the environment variables of the process, the
/// arguments of the command line and the placeholders of a value take part in
/// the binding like the properties of a property file do.
///
/// The generated implementation binds the properties with a [`Binder`] and
/// registers them in the [`ApplicationContext`](crate::ApplicationContext)
/// under [`name`](Self::name), which makes them available to the singletons
/// that depend on them.
pub trait ConfigurationProperties: DynClone + Send + Sync {
    /// Returns the name the properties are registered under.
    fn name(&self) -> &'static str;

    /// Returns the prefix the properties are bound from.
    fn prefix(&self) -> &'static str;

    /// Returns the properties the environment describes.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment the properties are bound from.
    ///
    /// # Errors
    ///
    /// Returns [`BindError`] when the properties cannot be bound.
    fn bind(environment: &dyn ConfigurableEnvironment) -> Result<Self, BindError>
    where
        Self: Sized;

    /// Binds the properties from the environment and registers them.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the properties are registered in.
    /// * `environment` - The environment the properties are bound from.
    ///
    /// # Errors
    ///
    /// Returns the error of the binding, or of the registration.
    fn register(
        &self,
        ctx: &mut dyn crate::ApplicationContext,
        environment: &dyn ConfigurableEnvironment,
    ) -> Result<(), Box<dyn Error>>;
}

dyn_clone::clone_trait_object!(ConfigurationProperties);

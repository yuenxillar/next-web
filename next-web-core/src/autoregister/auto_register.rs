use async_trait::async_trait;

use crate::{ApplicationContext, context::properties::ApplicationProperties, error::BoxError};

///
/// AutoRegister trait
///
/// This trait is used to register the singletons of a module in the application
/// context.
///
/// The `register` method is called by the framework once the context is
/// prepared. It takes two parameters:
/// - `ctx`: The application context.
/// - `properties`: The application properties.
///
/// The context is passed as `&mut dyn ApplicationContext`, so an implementation
/// does not depend on the context type of the application.
///
/// The `register` method returns a `Result` with an error type of
/// `Box<dyn std::error::Error + Send + Sync>`.
///
#[async_trait]
pub trait AutoRegister: Sync + Send {
    /// Get the name of the registration instance.
    ///
    /// This method is used to obtain the name of the registration instance.
    ///
    fn name(&self) -> &'static str;

    ///
    /// Register the singletons to the application context.
    ///
    /// This method is called by the framework to register the singletons.
    ///
    async fn register(
        &self,
        ctx: &mut dyn ApplicationContext,
        properties: &ApplicationProperties,
    ) -> Result<(), BoxError>;
}

// The auto registration of providers lives in the light context crate, together
// with the provider interface it registers. It is re-exported here for
// convenience.
pub use next_web_context::{
    AutoRegisterModule, ProviderRegister, auto_registered_providers, register_provider, submit,
};

use next_web_context::ApplicationContextExt;
use next_web_context::APPLICATION_ENVIRONMENT_SINGLETON_NAME;
use next_web_core::context::ConfigurationProperties;
use next_web_core::{
    async_trait, env::ConfigurableEnvironment,
    traits::config::auto_configuration::AutoConfiguration, ApplicationContext, Ordered,
};
use std::{error::Error, sync::Arc};

use crate::web::server::autoconfigure::ServerProperties;

#[derive(Debug, Clone)]
pub struct WebAutoConfiguration;

impl WebAutoConfiguration {
    /// Registers the configuration properties of the application.
    ///
    /// The properties every `#[configuration_properties]` type declares are
    /// bound from the environment of the application here, before any
    /// auto-configuration is applied, so that an auto-configuration that
    /// depends on them can resolve them.
    fn bind_configuration_properties(
        &self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn Error>> {
        let environment = ctx
            .get_singleton_option_with_name::<Arc<dyn ConfigurableEnvironment>>(
                APPLICATION_ENVIRONMENT_SINGLETON_NAME,
            )
            .map(Clone::clone)
            .ok_or_else(|| {
                format!(
                    "the environment of the application is not registered: \
                     {APPLICATION_ENVIRONMENT_SINGLETON_NAME}"
                )
            })?;

        for properties in ctx.resolve_by_type::<Box<dyn ConfigurationProperties>>() {
            properties.register(ctx, environment.as_ref())?;
        }

        Ok(())
    }

    /// Configures the locale resolver for the application context.
    #[cfg(feature = "enable-i18n")]
    fn locale_resolver(&self, ctx: &mut dyn ApplicationContext) {
        use crate::i18n::{AcceptHeaderLocaleResolver, LocaleResolver};

        if !ctx.contains_singleton_with_name::<std::sync::Arc<dyn LocaleResolver>>("localeResolver")
        {
            ctx.insert_singleton_with_name(
                std::sync::Arc::new(AcceptHeaderLocaleResolver::default()),
                "localeResolver",
            );
        }
    }

    /// Publishes the properties of the web server to the process.
    ///
    /// The properties of the server are registered as a singleton while the
    /// configuration properties are bound, and this hands them to
    /// [`ServerProperties::install_global`], so that the parts of the framework
    /// that are reached without an application context read the configuration
    /// of the application. Installing them twice is not an error: the first
    /// installation wins, which is the configuration of the first context of the
    /// process.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the properties were registered in.
    fn publish_server_properties(&self, ctx: &mut dyn ApplicationContext) {
        let Some(properties) = ctx
            .get_singleton_option_with_default_name::<ServerProperties>()
            .cloned()
        else {
            return;
        };

        if let Err(error) = properties.install_global() {
            tracing::debug!(
                error = %error,
                "the properties of the web server are installed already"
            );
        }
    }

    /// Auto-configures the application context by resolving and configuring all auto-configurations.
    ///
    /// The auto-configurations of an application are the types that implement
    /// [`AutoConfiguration`], which are contributed by the providers of the
    /// application:
    ///
    /// - the auto-configurations of a starter declare a `#[singleton(binds =
    ///   [Self::into_auto_configuration])]`,
    /// - the configuration classes of an application are declared with
    ///   `#[auto_configuration]`, which registers them the same way.
    ///
    /// They are applied in the order of [`Ordered`](next_web_core::Ordered), and
    /// a configuration whose condition does not hold is skipped.
    async fn run_auto_configurations(
        &mut self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn Error>> {
        let mut auto_configurations = ctx.resolve_by_type::<Box<dyn AutoConfiguration>>();
        auto_configurations.sort_by_key(|ac| ac.order());
        auto_configurations.retain(|ac| ac.matches(ctx));
        for mut auto_configuration in auto_configurations.into_iter() {
            auto_configuration.configure(ctx).await?;
        }

        // Resolve autoRegister
        // for auto_register in ctx
        //     .resolve_by_type::<Arc<dyn AutoRegister>>()
        //     .iter()
        //     .map(|s| s.as_ref())
        // {
        //     auto_register
        //         .register(ctx, application_properties)
        //         .await
        //         .map_err(|err| Into::<Box<dyn Error>>::into(err.to_string()))?;
        // }

        Ok(())
    }
}

#[async_trait]
impl AutoConfiguration for WebAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        self.bind_configuration_properties(ctx)?;
        self.publish_server_properties(ctx);
        self.run_auto_configurations(ctx).await?;

        // The tasks an application declared with `#[scheduled]` are resolved
        // after the auto-configurations, so that a task can depend on a
        // singleton an auto-configuration contributes.
        #[cfg(feature = "enable-scheduling")]
        crate::scheduling::SchedulingBootstrap::configure(ctx)
            .await
            .map_err(|error| -> Box<dyn Error> { error })?;

        #[cfg(feature = "enable-i18n")]
        self.locale_resolver(ctx);
        Ok(())
    }
}

impl Ordered for WebAutoConfiguration {
    fn order(&self) -> i32 {
        i32::MIN + 10
    }
}

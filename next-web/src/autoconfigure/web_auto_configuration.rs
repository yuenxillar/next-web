use next_web_context::APPLICATION_ENVIRONMENT_SINGLETON_NAME;
use next_web_context::ApplicationContextExt;
use next_web_context::{
    MESSAGE_SOURCE_SINGLETON_NAME, MessageSource, support::DelegatingMessageSource,
};
use next_web_core::context::ConfigurationProperties;
use next_web_core::{
    ApplicationContext, Ordered, async_trait,
    config::{AutoConfiguration, AutoConfigurationSorter, Configuration, ConfigurationSorter},
    env::ConfigurableEnvironment,
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
        let server_properties = ctx
            .get_singleton_option_with_default_name::<ServerProperties>()
            .cloned()
            .or_else(|| {
                ctx.get_singleton_option_with_name::<Arc<dyn ConfigurableEnvironment>>(
                    APPLICATION_ENVIRONMENT_SINGLETON_NAME,
                )
                .and_then(|env| ServerProperties::try_from(env.as_ref()).ok())
            });

        if let Err(error) = server_properties
            .map(ServerProperties::install_global)
            .transpose()
        {
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
    /// - the auto-configuration classes of an application are declared with
    ///   `#[auto_configuration]`, which registers them the same way.
    ///
    /// The configurations of an application, which are declared with
    /// `#[configuration]`, are applied before these, see
    /// [`Self::run_configurations`].
    ///
    /// They are applied in the order of [`Ordered`](next_web_core::Ordered),
    /// which the `before` and `after` names a configuration declares refine, and
    /// a configuration whose condition does not hold is skipped.
    async fn run_auto_configurations(
        &mut self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn Error>> {
        let auto_configurations = ctx.resolve_by_type::<Box<dyn AutoConfiguration>>();
        let mut auto_configurations = AutoConfigurationSorter::sort(auto_configurations)?;

        auto_configurations.retain(|configuration| configuration.matches(ctx));

        tracing::debug!(
            configurations = ?auto_configurations
                .iter()
                .map(|configuration| configuration.auto_configuration_name())
                .collect::<Vec<_>>(),
            "the auto-configurations of the application are applied in this order"
        );

        for auto_configuration in auto_configurations.into_iter().as_mut_slice() {
            auto_configuration.configure(ctx).await?;
        }

        Ok(())
    }

    /// Applies the configurations of the application.
    ///
    /// The configurations of an application are the types that implement
    /// [`Configuration`], which the configuration classes of the application are
    /// declared with: a class declared with `#[configuration]` registers one.
    ///
    /// They are applied **before** the auto-configurations, so that they can
    /// contribute the state an auto-configuration of a starter depends on, and
    /// they are applied in the order of [`Ordered`](next_web_core::Ordered),
    /// which the `before` and `after` names a configuration declares refine. A
    /// configuration whose condition does not hold is skipped.
    async fn run_configurations(
        &mut self,
        ctx: &mut dyn ApplicationContext,
    ) -> Result<(), Box<dyn Error>> {
        let configurations = ctx.resolve_by_type::<Box<dyn Configuration>>();
        let mut configurations = ConfigurationSorter::sort(configurations)?;

        configurations.retain(|configuration| configuration.matches(ctx));

        tracing::debug!(
            configurations = ?configurations
                .iter()
                .map(|configuration| configuration.configuration_name())
                .collect::<Vec<_>>(),
            "the configurations of the application are applied in this order"
        );

        for mut configuration in configurations {
            configuration.configure(ctx).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl AutoConfiguration for WebAutoConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        self.bind_configuration_properties(ctx)?;
        self.publish_server_properties(ctx);

        // The configurations of the application run first, so that a
        // configuration can contribute the state the auto-configurations of the
        // starters depend on.
        self.run_configurations(ctx).await?;

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

        // A context that neither an application nor an auto-configuration gave a
        // message source answers with the delegating source of the framework, so
        // that a component which requires a `MessageSource` always finds one. The
        // source resolves no message of its own, which is what a context without
        // message bundles answers with.
        if !ctx
            .contains_singleton_with_name::<Arc<dyn MessageSource>>(MESSAGE_SOURCE_SINGLETON_NAME)
        {
            ctx.insert_singleton_with_name::<Arc<dyn MessageSource>>(
                Arc::new(DelegatingMessageSource::default()),
                MESSAGE_SOURCE_SINGLETON_NAME,
            );
        }

        Ok(())
    }
}

impl Ordered for WebAutoConfiguration {
    fn order(&self) -> i32 {
        i32::MIN + 10
    }
}

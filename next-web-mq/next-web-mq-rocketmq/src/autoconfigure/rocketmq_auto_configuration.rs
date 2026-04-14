use std::error::Error;

use next_web_core::{
    ApplicationContext, async_trait, error::BoxError,
    traits::config::auto_configuration::AutoConfiguration,
};
use rudi_dev::singleton;
use tracing::info;

use crate::{
    autoconfigure::rocketmq_properties::RocketmqProperties,
    config::{
        default_rocketmq_listener_registry::DefaultRocketmqListenerRegistry,
        rocketmq_configurer::RocketmqConfigurer,
    },
    service::rocketmq_template::RocketmqTemplate,
};

/// Auto-configuration entry point for the RocketMQ starter.
///
/// The framework instantiates this type as a singleton, collects all
/// [`RocketmqConfigurer`] implementations, builds a listener registry, and
/// finally publishes a ready-to-inject [`RocketmqTemplate`] into the
/// application context.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct RocketmqAutoConfiguration {
    /// Resolved RocketMQ properties loaded from configuration files.
    pub rocketmq_properties: RocketmqProperties,

    #[autowired(vec)]
    configurers: Vec<Box<dyn RocketmqConfigurer>>,
}

impl RocketmqAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for RocketmqAutoConfiguration {
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {
        let mut registry = DefaultRocketmqListenerRegistry::default();
        for configurer in self.configurers.iter_mut() {
            configurer.register_listeners(ctx, &mut registry);
        }

        let rocketmq_template = RocketmqTemplate::new(
            self.rocketmq_properties.clone(),
            registry.registrations().to_vec(),
        );

        info!(
            "RocketMQ starter initialized, name_servers={:?}, listeners={}",
            rocketmq_template.properties().name_servers(),
            rocketmq_template.registrations().len()
        );

        ctx.insert_singleton_with_default_name(rocketmq_template);
        Ok(())
    }
}

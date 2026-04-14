use std::error::Error;

use next_web_core::{
    ApplicationContext, async_trait, traits::config::auto_configuration::AutoConfiguration,
};
use rudi_dev::singleton;

use crate::{
    autoconfigure::rabbitmq_properties::RabbitmqProperties,
    config::{
        default_rabbitmq_listener_registry::DefaultRabbitmqListenerRegistry,
        rabbitmq_configurer::RabbitmqConfigurer,
    },
    service::rabbitmq_service::RabbitmqService,
};

/// Auto-configuration for RabbitMQ.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct RabbitmqAutoConfiguration {
    pub rabbitmq_properties: RabbitmqProperties,

    #[autowired(vec)]
    configurers: Vec<Box<dyn RabbitmqConfigurer>>,
}

impl RabbitmqAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for RabbitmqAutoConfiguration {
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {
        let mut registry = DefaultRabbitmqListenerRegistry::default();
        for configurer in self.configurers.iter_mut() {
            configurer.register_listeners(ctx, &mut registry);
        }

        let rabbitmq_service =
            RabbitmqService::new(self.rabbitmq_properties.clone(), registry.endpoints()).await?;

        rabbitmq_service
            .start_listeners(registry.registrations())
            .await?;

        ctx.insert_singleton_with_default_name(rabbitmq_service);
        Ok(())
    }
}

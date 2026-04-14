use std::error::Error;

use next_web_core::{
    ApplicationContext, async_trait, error::BoxError,
    traits::config::auto_configuration::AutoConfiguration,
};
use rudi_dev::singleton;
use tracing::info;

use crate::{
    autoconfigure::kafka_properties::KafkaProperties,
    config::{
        default_kafka_listener_registry::DefaultKafkaListenerRegistry,
        kafka_configurer::KafkaConfigurer,
    },
    service::kafka_template::KafkaTemplate,
};

/// Auto-configuration for Kafka starter components.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct KafkaAutoConfiguration {
    pub kafka_properties: KafkaProperties,

    #[autowired(vec)]
    configurers: Vec<Box<dyn KafkaConfigurer>>,
}

impl KafkaAutoConfiguration {
    fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for KafkaAutoConfiguration {
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>> {
        let mut registry = DefaultKafkaListenerRegistry::default();
        for configurer in self.configurers.iter_mut() {
            configurer.register_listeners(ctx, &mut registry);
        }

        let kafka_template = KafkaTemplate::new(
            self.kafka_properties.clone(),
            registry.registrations().to_vec(),
        );

        info!(
            "Kafka starter initialized, bootstrap_servers={:?}, listeners={}",
            kafka_template.properties().bootstrap_servers(),
            kafka_template.registrations().len()
        );

        ctx.insert_singleton_with_default_name(kafka_template);
        Ok(())
    }
}

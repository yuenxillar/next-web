use next_web_core::{
    async_trait, context::properties::ApplicationProperties, interface::singleton::Singleton,
    ApplicationContext, AutoRegister,
};
use rudi_dev::SingleOwner;

use crate::rabbitmq::{
    core::{bind_exchange::BindExchangeBuilder, listener::rabbit_listener::RabbitListener},
    properties::rabbitmq_properties::RabbitMQClientProperties,
    service::rabbitmq_service::RabbitmqService,
};

#[SingleOwner(binds = [Self::into_auto_register])]
pub struct RabbitmqServiceAutoRegister(pub RabbitMQClientProperties);

impl RabbitmqServiceAutoRegister {
    fn into_auto_register(self) -> Box<dyn AutoRegister> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoRegister for RabbitmqServiceAutoRegister {
    fn registered_name(&self) -> &'static str {
        ""
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bind_exchange = ctx.resolve::<Box<dyn BindExchangeBuilder>>().value();

        let properties = self.0.clone();

        let rabbitmq_service = RabbitmqService::new(properties, bind_exchange).await;

        let consumer: Vec<Box<dyn RabbitListener>> = ctx.resolve_by_type::<Box<dyn RabbitListener>>();

        rabbitmq_service.spawn_consumer(consumer).await;
        ctx.insert_singleton_with_name(rabbitmq_service, self.singleton_name());
        Ok(())
    }
}

use next_web_core::{
    async_trait, context::properties::ApplicationProperties, ApplicationContext, AutoRegister,
};
use rudi_dev::SingleOwner;

use crate::rabbitmq::{
    bind_exchange::BindExchangeBuilder, properties::rabbitmq_properties::RabbitMQClientProperties,
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
    fn singleton_name(&self) -> &'static str {
        "rabbitmqService"
    }

    async fn register(
        &self,
        ctx: &mut ApplicationContext,
        _properties: &ApplicationProperties,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bind_exchange = ctx.resolve::<Box<dyn BindExchangeBuilder>>().value();

        let properties = self.0.clone();

        let rabbitmq_manager = RabbitmqService::new(properties, bind_exchange).await;
        ctx.insert_singleton_with_name(rabbitmq_manager, self.singleton_name());

        Ok(())
    }
}

use next_web_mq::amqprs::channel::Channel;
use next_web_mq::rabbitmq::{
    bind_exchange::BindExchange, core::client_properties::RabbitMQClientProperties,
};

#[derive(Clone)]
pub struct RabbiqMQManager {
    channel: Channel,
    options: RabbitMQClientProperties,
    bind_exchange: Vec<BindExchange>,
}

impl RabbiqMQManager {
    pub fn options(&self) -> &RabbitMQClientProperties {
        &self.options
    }
    pub fn bind_exchange(&self) -> &Vec<BindExchange> {
        &self.bind_exchange
    }

    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn new(
        options: RabbitMQClientProperties,
        bind_exchange: Vec<BindExchange>,
        channel: Channel,
    ) -> Self {
        Self {
            options,
            bind_exchange,
            channel,
        }
    }
}

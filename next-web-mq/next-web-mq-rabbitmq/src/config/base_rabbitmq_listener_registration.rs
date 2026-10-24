use std::sync::Arc;

use crate::{
    config::rabbitmq_listener_registration::RabbitmqListenerRegistration,
    core::{
        binding::{RabbitmqEndpoint, RabbitmqListenerEndpoint},
        listener::rabbit_listener::RabbitListener,
    },
};

/// Base registration object that stores RabbitMQ listener endpoint options.
#[derive(Clone)]
pub struct BaseRabbitmqListenerRegistration {
    listener_endpoint: RabbitmqListenerEndpoint,
}

impl BaseRabbitmqListenerRegistration {
    pub fn new(
        listener: Arc<dyn RabbitListener>,
        queue_name: String,
        exchange_name: String,
        routing_key: String,
    ) -> Self {
        Self {
            listener_endpoint: RabbitmqListenerEndpoint::new(
                listener,
                RabbitmqEndpoint::new(queue_name, exchange_name, routing_key),
            ),
        }
    }

    pub fn endpoint(&self) -> &RabbitmqEndpoint {
        self.listener_endpoint.endpoint()
    }

    pub fn listener(&self) -> Arc<dyn RabbitListener> {
        self.listener_endpoint.listener()
    }

    pub fn consumer_tag_value(&self) -> &str {
        self.listener_endpoint.consumer_tag()
    }
}

impl RabbitmqListenerRegistration for BaseRabbitmqListenerRegistration {
    fn consumer_tag(&mut self, consumer_tag: String) -> &mut dyn RabbitmqListenerRegistration {
        self.listener_endpoint.set_consumer_tag(consumer_tag);
        self
    }

    fn exchange_type(&mut self, exchange_type: String) -> &mut dyn RabbitmqListenerRegistration {
        self.listener_endpoint.set_exchange_type(exchange_type);
        self
    }

    fn exchange_durable(&mut self, durable: bool) -> &mut dyn RabbitmqListenerRegistration {
        self.listener_endpoint.set_exchange_durable(durable);
        self
    }

    fn queue_durable(&mut self, durable: bool) -> &mut dyn RabbitmqListenerRegistration {
        self.listener_endpoint.set_queue_durable(durable);
        self
    }
}

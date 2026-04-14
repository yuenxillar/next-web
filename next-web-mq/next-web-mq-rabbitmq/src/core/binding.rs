use std::sync::Arc;

use crate::core::listener::rabbit_listener::RabbitListener;

/// Declarative queue and exchange binding metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RabbitmqEndpoint {
    queue_name: String,
    exchange_name: String,
    routing_key: String,
    exchange_type: String,
    exchange_durable: bool,
    queue_durable: bool,
}

impl RabbitmqEndpoint {
    pub fn new(
        queue_name: impl Into<String>,
        exchange_name: impl Into<String>,
        routing_key: impl Into<String>,
    ) -> Self {
        Self {
            queue_name: queue_name.into(),
            exchange_name: exchange_name.into(),
            routing_key: routing_key.into(),
            exchange_type: "direct".into(),
            exchange_durable: true,
            queue_durable: true,
        }
    }

    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }

    pub fn exchange_name(&self) -> &str {
        &self.exchange_name
    }

    pub fn routing_key(&self) -> &str {
        &self.routing_key
    }

    pub fn exchange_type(&self) -> &str {
        &self.exchange_type
    }

    pub fn exchange_durable(&self) -> bool {
        self.exchange_durable
    }

    pub fn queue_durable(&self) -> bool {
        self.queue_durable
    }

    pub fn set_exchange_type(&mut self, exchange_type: impl Into<String>) {
        self.exchange_type = exchange_type.into();
    }

    pub fn set_exchange_durable(&mut self, durable: bool) {
        self.exchange_durable = durable;
    }

    pub fn set_queue_durable(&mut self, durable: bool) {
        self.queue_durable = durable;
    }
}

/// Listener endpoint that combines a binding and a listener instance.
#[derive(Clone)]
pub struct RabbitmqListenerEndpoint {
    listener: Arc<dyn RabbitListener>,
    endpoint: RabbitmqEndpoint,
    consumer_tag: String,
}

impl RabbitmqListenerEndpoint {
    pub fn new(listener: Arc<dyn RabbitListener>, endpoint: RabbitmqEndpoint) -> Self {
        Self {
            listener,
            endpoint,
            consumer_tag: String::new(),
        }
    }

    pub fn with_consumer_tag(mut self, consumer_tag: String) -> Self {
        self.consumer_tag = consumer_tag;
        self
    }

    pub fn endpoint(&self) -> &RabbitmqEndpoint {
        &self.endpoint
    }

    pub fn listener(&self) -> Arc<dyn RabbitListener> {
        self.listener.clone()
    }

    pub fn consumer_tag(&self) -> &str {
        &self.consumer_tag
    }

    pub fn set_consumer_tag(&mut self, consumer_tag: String) {
        self.consumer_tag = consumer_tag;
    }

    pub fn set_exchange_type(&mut self, exchange_type: String) {
        self.endpoint.set_exchange_type(exchange_type);
    }

    pub fn set_exchange_durable(&mut self, durable: bool) {
        self.endpoint.set_exchange_durable(durable);
    }

    pub fn set_queue_durable(&mut self, durable: bool) {
        self.endpoint.set_queue_durable(durable);
    }
}

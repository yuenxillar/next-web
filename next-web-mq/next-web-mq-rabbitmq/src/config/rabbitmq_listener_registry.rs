use std::sync::Arc;

use crate::{
    config::rabbitmq_listener_registration::RabbitmqListenerRegistration,
    core::listener::rabbit_listener::RabbitListener,
};

/// Registry for RabbitMQ listener endpoints.
pub trait RabbitmqListenerRegistry {
    fn register_listener<'a>(
        &'a mut self,
        listener: Arc<dyn RabbitListener>,
        queue_name: String,
        exchange_name: String,
        routing_key: String,
    ) -> &'a mut dyn RabbitmqListenerRegistration;
}

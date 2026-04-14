use std::sync::Arc;

use crate::{
    config::{
        base_rabbitmq_listener_registration::BaseRabbitmqListenerRegistration,
        default_rabbitmq_listener_registration::DefaultRabbitmqListenerRegistration,
        rabbitmq_listener_registration::RabbitmqListenerRegistration,
        rabbitmq_listener_registry::RabbitmqListenerRegistry,
    },
    core::{binding::RabbitmqEndpoint, listener::rabbit_listener::RabbitListener},
};

/// Default registry used by auto-configuration to collect listener endpoints.
#[derive(Default)]
pub struct DefaultRabbitmqListenerRegistry {
    registrations: Vec<DefaultRabbitmqListenerRegistration>,
}

impl DefaultRabbitmqListenerRegistry {
    pub fn registrations(&self) -> &[DefaultRabbitmqListenerRegistration] {
        self.registrations.as_slice()
    }

    pub fn endpoints(&self) -> Vec<RabbitmqEndpoint> {
        self.registrations
            .iter()
            .map(|registration| registration.endpoint().clone())
            .collect()
    }
}

impl RabbitmqListenerRegistry for DefaultRabbitmqListenerRegistry {
    fn register_listener<'a>(
        &'a mut self,
        listener: Arc<dyn RabbitListener>,
        queue_name: String,
        exchange_name: String,
        routing_key: String,
    ) -> &'a mut dyn RabbitmqListenerRegistration {
        let registration = DefaultRabbitmqListenerRegistration::new(
            BaseRabbitmqListenerRegistration::new(listener, queue_name, exchange_name, routing_key),
        );
        self.registrations.push(registration);
        self.registrations
            .last_mut()
            .expect("registration must exist")
    }
}

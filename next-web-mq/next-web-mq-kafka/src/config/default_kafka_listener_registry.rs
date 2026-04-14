use std::sync::Arc;

use crate::{
    config::{
        base_kafka_listener_registration::BaseKafkaListenerRegistration,
        default_kafka_listener_registration::DefaultKafkaListenerRegistration,
        kafka_listener_registration::KafkaListenerRegistration,
        kafka_listener_registry::KafkaListenerRegistry,
    },
    core::{endpoint::KafkaEndpoint, listener::kafka_listener::KafkaListener},
};

/// Default registry used by auto-configuration to collect Kafka listener endpoints.
#[derive(Default)]
pub struct DefaultKafkaListenerRegistry {
    registrations: Vec<DefaultKafkaListenerRegistration>,
}

impl DefaultKafkaListenerRegistry {
    pub fn registrations(&self) -> &[DefaultKafkaListenerRegistration] {
        self.registrations.as_slice()
    }

    pub fn endpoints(&self) -> Vec<KafkaEndpoint> {
        self.registrations
            .iter()
            .map(|registration| registration.endpoint().clone())
            .collect()
    }
}

impl KafkaListenerRegistry for DefaultKafkaListenerRegistry {
    fn register_listener<'a>(
        &'a mut self,
        listener: Arc<dyn KafkaListener>,
        topic: String,
    ) -> &'a mut dyn KafkaListenerRegistration {
        let registration = DefaultKafkaListenerRegistration::new(
            BaseKafkaListenerRegistration::new(listener, topic),
        );
        self.registrations.push(registration);
        self.registrations
            .last_mut()
            .expect("registration must exist")
    }
}

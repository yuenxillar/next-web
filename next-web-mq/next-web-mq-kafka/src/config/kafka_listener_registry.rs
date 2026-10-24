use std::sync::Arc;

use crate::{
    config::kafka_listener_registration::KafkaListenerRegistration,
    core::listener::kafka_listener::KafkaListener,
};

/// Registry used by the starter to collect Kafka listeners.
pub trait KafkaListenerRegistry {
    fn register_listener<'a>(
        &'a mut self,
        listener: Arc<dyn KafkaListener>,
        topic: String,
    ) -> &'a mut dyn KafkaListenerRegistration;
}

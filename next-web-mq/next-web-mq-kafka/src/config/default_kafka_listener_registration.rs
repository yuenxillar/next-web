use std::ops::{Deref, DerefMut};

use crate::config::{
    base_kafka_listener_registration::BaseKafkaListenerRegistration,
    kafka_listener_registration::KafkaListenerRegistration,
};

/// Default listener registration wrapper used by the auto-configuration flow.
#[derive(Clone)]
pub struct DefaultKafkaListenerRegistration {
    base: BaseKafkaListenerRegistration,
}

impl DefaultKafkaListenerRegistration {
    pub fn new(base: BaseKafkaListenerRegistration) -> Self {
        Self { base }
    }
}

impl KafkaListenerRegistration for DefaultKafkaListenerRegistration {
    fn group_id(&mut self, group_id: String) -> &mut dyn KafkaListenerRegistration {
        self.base.group_id(group_id)
    }

    fn client_id_prefix(&mut self, client_id_prefix: String) -> &mut dyn KafkaListenerRegistration {
        self.base.client_id_prefix(client_id_prefix)
    }

    fn concurrency(&mut self, concurrency: u16) -> &mut dyn KafkaListenerRegistration {
        self.base.concurrency(concurrency)
    }

    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn KafkaListenerRegistration {
        self.base.auto_startup(auto_startup)
    }

    fn poll_timeout_ms(&mut self, poll_timeout_ms: u64) -> &mut dyn KafkaListenerRegistration {
        self.base.poll_timeout_ms(poll_timeout_ms)
    }
}

impl Deref for DefaultKafkaListenerRegistration {
    type Target = BaseKafkaListenerRegistration;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DefaultKafkaListenerRegistration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

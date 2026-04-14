use std::sync::Arc;

use crate::{
    config::kafka_listener_registration::KafkaListenerRegistration,
    core::{
        endpoint::{KafkaEndpoint, KafkaListenerEndpoint},
        listener::kafka_listener::KafkaListener,
    },
};

/// Base registration that stores listener metadata before a transport binds it.
#[derive(Clone)]
pub struct BaseKafkaListenerRegistration {
    listener_endpoint: KafkaListenerEndpoint,
}

impl BaseKafkaListenerRegistration {
    pub fn new(listener: Arc<dyn KafkaListener>, topic: String) -> Self {
        Self {
            listener_endpoint: KafkaListenerEndpoint::new(listener, KafkaEndpoint::new(topic)),
        }
    }

    pub fn endpoint(&self) -> &KafkaEndpoint {
        self.listener_endpoint.endpoint()
    }

    pub fn listener(&self) -> Arc<dyn KafkaListener> {
        self.listener_endpoint.listener()
    }

    pub fn group_id_value(&self) -> Option<&str> {
        self.listener_endpoint.group_id()
    }

    pub fn client_id_prefix_value(&self) -> Option<&str> {
        self.listener_endpoint.client_id_prefix()
    }

    pub fn concurrency_value(&self) -> u16 {
        self.listener_endpoint.concurrency()
    }

    pub fn auto_startup_value(&self) -> bool {
        self.listener_endpoint.auto_startup()
    }

    pub fn poll_timeout_ms_value(&self) -> u64 {
        self.listener_endpoint.poll_timeout_ms()
    }
}

impl KafkaListenerRegistration for BaseKafkaListenerRegistration {
    fn group_id(&mut self, group_id: String) -> &mut dyn KafkaListenerRegistration {
        self.listener_endpoint.set_group_id(group_id);
        self
    }

    fn client_id_prefix(&mut self, client_id_prefix: String) -> &mut dyn KafkaListenerRegistration {
        self.listener_endpoint
            .set_client_id_prefix(client_id_prefix);
        self
    }

    fn concurrency(&mut self, concurrency: u16) -> &mut dyn KafkaListenerRegistration {
        self.listener_endpoint.set_concurrency(concurrency);
        self
    }

    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn KafkaListenerRegistration {
        self.listener_endpoint.set_auto_startup(auto_startup);
        self
    }

    fn poll_timeout_ms(&mut self, poll_timeout_ms: u64) -> &mut dyn KafkaListenerRegistration {
        self.listener_endpoint.set_poll_timeout_ms(poll_timeout_ms);
        self
    }
}

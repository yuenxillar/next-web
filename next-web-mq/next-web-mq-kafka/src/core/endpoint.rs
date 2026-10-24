use std::sync::Arc;

use crate::core::listener::kafka_listener::KafkaListener;

/// Declarative Kafka topic binding metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KafkaEndpoint {
    topic: String,
    group_id: Option<String>,
    client_id_prefix: Option<String>,
    concurrency: u16,
    auto_startup: bool,
    poll_timeout_ms: u64,
}

impl KafkaEndpoint {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            group_id: None,
            client_id_prefix: None,
            concurrency: 1,
            auto_startup: true,
            poll_timeout_ms: 1_000,
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn group_id(&self) -> Option<&str> {
        self.group_id.as_deref()
    }

    pub fn client_id_prefix(&self) -> Option<&str> {
        self.client_id_prefix.as_deref()
    }

    pub fn concurrency(&self) -> u16 {
        self.concurrency
    }

    pub fn auto_startup(&self) -> bool {
        self.auto_startup
    }

    pub fn poll_timeout_ms(&self) -> u64 {
        self.poll_timeout_ms
    }

    pub fn set_group_id(&mut self, group_id: impl Into<String>) {
        self.group_id = Some(group_id.into());
    }

    pub fn set_client_id_prefix(&mut self, client_id_prefix: impl Into<String>) {
        self.client_id_prefix = Some(client_id_prefix.into());
    }

    pub fn set_concurrency(&mut self, concurrency: u16) {
        self.concurrency = concurrency.max(1);
    }

    pub fn set_auto_startup(&mut self, auto_startup: bool) {
        self.auto_startup = auto_startup;
    }

    pub fn set_poll_timeout_ms(&mut self, poll_timeout_ms: u64) {
        self.poll_timeout_ms = poll_timeout_ms.max(1);
    }
}

/// Listener endpoint that combines Kafka metadata and a listener instance.
#[derive(Clone)]
pub struct KafkaListenerEndpoint {
    listener: Arc<dyn KafkaListener>,
    endpoint: KafkaEndpoint,
}

impl KafkaListenerEndpoint {
    pub fn new(listener: Arc<dyn KafkaListener>, endpoint: KafkaEndpoint) -> Self {
        Self { listener, endpoint }
    }

    pub fn endpoint(&self) -> &KafkaEndpoint {
        &self.endpoint
    }

    pub fn listener(&self) -> Arc<dyn KafkaListener> {
        self.listener.clone()
    }

    pub fn group_id(&self) -> Option<&str> {
        self.endpoint.group_id()
    }

    pub fn client_id_prefix(&self) -> Option<&str> {
        self.endpoint.client_id_prefix()
    }

    pub fn concurrency(&self) -> u16 {
        self.endpoint.concurrency()
    }

    pub fn auto_startup(&self) -> bool {
        self.endpoint.auto_startup()
    }

    pub fn poll_timeout_ms(&self) -> u64 {
        self.endpoint.poll_timeout_ms()
    }

    pub fn set_group_id(&mut self, group_id: impl Into<String>) {
        self.endpoint.set_group_id(group_id);
    }

    pub fn set_client_id_prefix(&mut self, client_id_prefix: impl Into<String>) {
        self.endpoint.set_client_id_prefix(client_id_prefix);
    }

    pub fn set_concurrency(&mut self, concurrency: u16) {
        self.endpoint.set_concurrency(concurrency);
    }

    pub fn set_auto_startup(&mut self, auto_startup: bool) {
        self.endpoint.set_auto_startup(auto_startup);
    }

    pub fn set_poll_timeout_ms(&mut self, poll_timeout_ms: u64) {
        self.endpoint.set_poll_timeout_ms(poll_timeout_ms);
    }
}

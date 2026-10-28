use std::sync::Arc;

use crate::{
    config::rocketmq_listener_registration::RocketmqListenerRegistration,
    core::listener::rocketmq_listener::RocketmqListener,
};

/// Registry used by the starter to collect RocketMQ listeners.
pub trait RocketmqListenerRegistry {
    /// Registers a listener for the provided topic and returns a fluent
    /// registration handle for additional customization.
    fn register_listener<'a>(
        &'a mut self,
        listener: Arc<dyn RocketmqListener>,
        topic: String,
    ) -> &'a mut dyn RocketmqListenerRegistration;
}

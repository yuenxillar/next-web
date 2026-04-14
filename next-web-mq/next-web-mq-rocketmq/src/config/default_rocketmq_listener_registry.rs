use std::sync::Arc;

use crate::{
    config::{
        base_rocketmq_listener_registration::BaseRocketmqListenerRegistration,
        default_rocketmq_listener_registration::DefaultRocketmqListenerRegistration,
        rocketmq_listener_registration::RocketmqListenerRegistration,
        rocketmq_listener_registry::RocketmqListenerRegistry,
    },
    core::{endpoint::RocketmqEndpoint, listener::rocketmq_listener::RocketmqListener},
};

/// Default registry used by auto-configuration to collect RocketMQ listener
/// endpoints.
#[derive(Default)]
pub struct DefaultRocketmqListenerRegistry {
    registrations: Vec<DefaultRocketmqListenerRegistration>,
}

impl DefaultRocketmqListenerRegistry {
    /// Returns the collected listener registrations.
    pub fn registrations(&self) -> &[DefaultRocketmqListenerRegistration] {
        self.registrations.as_slice()
    }

    /// Returns a cloned snapshot of all declared endpoints.
    pub fn endpoints(&self) -> Vec<RocketmqEndpoint> {
        self.registrations
            .iter()
            .map(|registration| registration.endpoint().clone())
            .collect()
    }
}

impl RocketmqListenerRegistry for DefaultRocketmqListenerRegistry {
    fn register_listener<'a>(
        &'a mut self,
        listener: Arc<dyn RocketmqListener>,
        topic: String,
    ) -> &'a mut dyn RocketmqListenerRegistration {
        let registration = DefaultRocketmqListenerRegistration::new(
            BaseRocketmqListenerRegistration::new(listener, topic),
        );
        self.registrations.push(registration);
        self.registrations
            .last_mut()
            .expect("registration must exist")
    }
}

use std::ops::{Deref, DerefMut};

use crate::config::{
    base_rocketmq_listener_registration::BaseRocketmqListenerRegistration,
    rocketmq_listener_registration::RocketmqListenerRegistration,
};
use crate::core::endpoint::RocketmqMessageModel;

/// Default listener registration wrapper used by the auto-configuration flow.
#[derive(Clone)]
pub struct DefaultRocketmqListenerRegistration {
    base: BaseRocketmqListenerRegistration,
}

impl DefaultRocketmqListenerRegistration {
    /// Wraps a base registration so the default registry can store it.
    pub fn new(base: BaseRocketmqListenerRegistration) -> Self {
        Self { base }
    }
}

impl RocketmqListenerRegistration for DefaultRocketmqListenerRegistration {
    fn consumer_group(&mut self, consumer_group: String) -> &mut dyn RocketmqListenerRegistration {
        self.base.consumer_group(consumer_group)
    }

    fn selector_expression(
        &mut self,
        selector_expression: String,
    ) -> &mut dyn RocketmqListenerRegistration {
        self.base.selector_expression(selector_expression)
    }

    fn message_model(
        &mut self,
        message_model: RocketmqMessageModel,
    ) -> &mut dyn RocketmqListenerRegistration {
        self.base.message_model(message_model)
    }

    fn orderly(&mut self, orderly: bool) -> &mut dyn RocketmqListenerRegistration {
        self.base.orderly(orderly)
    }

    fn consume_batch_size(
        &mut self,
        consume_batch_size: u32,
    ) -> &mut dyn RocketmqListenerRegistration {
        self.base.consume_batch_size(consume_batch_size)
    }

    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn RocketmqListenerRegistration {
        self.base.auto_startup(auto_startup)
    }
}

impl Deref for DefaultRocketmqListenerRegistration {
    type Target = BaseRocketmqListenerRegistration;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DefaultRocketmqListenerRegistration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

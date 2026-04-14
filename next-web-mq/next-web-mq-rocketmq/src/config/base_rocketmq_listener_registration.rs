use std::sync::Arc;

use crate::{
    config::rocketmq_listener_registration::RocketmqListenerRegistration,
    core::{
        endpoint::{RocketmqEndpoint, RocketmqListenerEndpoint, RocketmqMessageModel},
        listener::rocketmq_listener::RocketmqListener,
    },
};

/// Base registration that stores listener metadata before a RocketMQ client
/// adapter binds it.
#[derive(Clone)]
pub struct BaseRocketmqListenerRegistration {
    listener_endpoint: RocketmqListenerEndpoint,
}

impl BaseRocketmqListenerRegistration {
    /// Creates a new registration for the given listener and topic.
    pub fn new(listener: Arc<dyn RocketmqListener>, topic: String) -> Self {
        Self {
            listener_endpoint: RocketmqListenerEndpoint::new(
                listener,
                RocketmqEndpoint::new(topic),
            ),
        }
    }

    /// Returns the declarative endpoint metadata associated with this
    /// registration.
    pub fn endpoint(&self) -> &RocketmqEndpoint {
        self.listener_endpoint.endpoint()
    }

    /// Returns the registered listener instance.
    pub fn listener(&self) -> Arc<dyn RocketmqListener> {
        self.listener_endpoint.listener()
    }

    /// Returns the effective consumer group override, if any.
    pub fn consumer_group_value(&self) -> Option<&str> {
        self.listener_endpoint.consumer_group()
    }

    /// Returns the message selector expression, if any.
    pub fn selector_expression_value(&self) -> Option<&str> {
        self.listener_endpoint.selector_expression()
    }

    /// Returns the selected message model.
    pub fn message_model_value(&self) -> RocketmqMessageModel {
        self.listener_endpoint.message_model()
    }

    /// Returns whether the listener should consume messages in order.
    pub fn orderly_value(&self) -> bool {
        self.listener_endpoint.orderly()
    }

    /// Returns the requested consumer batch size.
    pub fn consume_batch_size_value(&self) -> u32 {
        self.listener_endpoint.consume_batch_size()
    }

    /// Returns whether the listener should start automatically.
    pub fn auto_startup_value(&self) -> bool {
        self.listener_endpoint.auto_startup()
    }
}

impl RocketmqListenerRegistration for BaseRocketmqListenerRegistration {
    fn consumer_group(&mut self, consumer_group: String) -> &mut dyn RocketmqListenerRegistration {
        self.listener_endpoint.set_consumer_group(consumer_group);
        self
    }

    fn selector_expression(
        &mut self,
        selector_expression: String,
    ) -> &mut dyn RocketmqListenerRegistration {
        self.listener_endpoint
            .set_selector_expression(selector_expression);
        self
    }

    fn message_model(
        &mut self,
        message_model: RocketmqMessageModel,
    ) -> &mut dyn RocketmqListenerRegistration {
        self.listener_endpoint.set_message_model(message_model);
        self
    }

    fn orderly(&mut self, orderly: bool) -> &mut dyn RocketmqListenerRegistration {
        self.listener_endpoint.set_orderly(orderly);
        self
    }

    fn consume_batch_size(
        &mut self,
        consume_batch_size: u32,
    ) -> &mut dyn RocketmqListenerRegistration {
        self.listener_endpoint
            .set_consume_batch_size(consume_batch_size);
        self
    }

    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn RocketmqListenerRegistration {
        self.listener_endpoint.set_auto_startup(auto_startup);
        self
    }
}

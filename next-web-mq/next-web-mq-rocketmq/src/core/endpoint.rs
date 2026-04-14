use std::sync::Arc;

use crate::core::listener::rocketmq_listener::RocketmqListener;

/// Message consumption models supported by RocketMQ.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RocketmqMessageModel {
    /// Each consumer group member receives a subset of the messages.
    #[default]
    Clustering,

    /// Each consumer group member receives the full message stream.
    Broadcasting,
}

/// Declarative RocketMQ topic binding metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RocketmqEndpoint {
    topic: String,
    consumer_group: Option<String>,
    selector_expression: Option<String>,
    message_model: RocketmqMessageModel,
    orderly: bool,
    consume_batch_size: u32,
    auto_startup: bool,
}

impl RocketmqEndpoint {
    /// Creates a new endpoint bound to the given topic.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            consumer_group: None,
            selector_expression: None,
            message_model: RocketmqMessageModel::Clustering,
            orderly: false,
            consume_batch_size: 1,
            auto_startup: true,
        }
    }

    /// Returns the subscribed topic.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns the optional consumer group override.
    pub fn consumer_group(&self) -> Option<&str> {
        self.consumer_group.as_deref()
    }

    /// Returns the optional selector expression.
    pub fn selector_expression(&self) -> Option<&str> {
        self.selector_expression.as_deref()
    }

    /// Returns the configured message model.
    pub fn message_model(&self) -> RocketmqMessageModel {
        self.message_model
    }

    /// Returns whether consumption must preserve message order.
    pub fn orderly(&self) -> bool {
        self.orderly
    }

    /// Returns the maximum number of messages delivered in a batch.
    pub fn consume_batch_size(&self) -> u32 {
        self.consume_batch_size
    }

    /// Returns whether this endpoint starts automatically during boot.
    pub fn auto_startup(&self) -> bool {
        self.auto_startup
    }

    /// Sets the consumer group override.
    pub fn set_consumer_group(&mut self, consumer_group: impl Into<String>) {
        self.consumer_group = Some(consumer_group.into());
    }

    /// Sets the selector expression used to filter messages.
    pub fn set_selector_expression(&mut self, selector_expression: impl Into<String>) {
        self.selector_expression = Some(selector_expression.into());
    }

    /// Sets the message model used by this endpoint.
    pub fn set_message_model(&mut self, message_model: RocketmqMessageModel) {
        self.message_model = message_model;
    }

    /// Sets whether messages should be consumed in order.
    pub fn set_orderly(&mut self, orderly: bool) {
        self.orderly = orderly;
    }

    /// Sets the maximum number of messages delivered in a batch.
    ///
    /// Values smaller than `1` are normalized to `1`.
    pub fn set_consume_batch_size(&mut self, consume_batch_size: u32) {
        self.consume_batch_size = consume_batch_size.max(1);
    }

    /// Sets whether this endpoint should start automatically.
    pub fn set_auto_startup(&mut self, auto_startup: bool) {
        self.auto_startup = auto_startup;
    }
}

/// Listener endpoint that combines RocketMQ metadata and a listener instance.
#[derive(Clone)]
pub struct RocketmqListenerEndpoint {
    listener: Arc<dyn RocketmqListener>,
    endpoint: RocketmqEndpoint,
}

impl RocketmqListenerEndpoint {
    /// Creates a new listener endpoint.
    pub fn new(listener: Arc<dyn RocketmqListener>, endpoint: RocketmqEndpoint) -> Self {
        Self { listener, endpoint }
    }

    /// Returns the declarative endpoint metadata.
    pub fn endpoint(&self) -> &RocketmqEndpoint {
        &self.endpoint
    }

    /// Returns the listener instance.
    pub fn listener(&self) -> Arc<dyn RocketmqListener> {
        self.listener.clone()
    }

    /// Returns the effective consumer group override.
    pub fn consumer_group(&self) -> Option<&str> {
        self.endpoint.consumer_group()
    }

    /// Returns the selector expression.
    pub fn selector_expression(&self) -> Option<&str> {
        self.endpoint.selector_expression()
    }

    /// Returns the selected message model.
    pub fn message_model(&self) -> RocketmqMessageModel {
        self.endpoint.message_model()
    }

    /// Returns whether messages should be consumed in order.
    pub fn orderly(&self) -> bool {
        self.endpoint.orderly()
    }

    /// Returns the requested consume batch size.
    pub fn consume_batch_size(&self) -> u32 {
        self.endpoint.consume_batch_size()
    }

    /// Returns whether the endpoint starts automatically.
    pub fn auto_startup(&self) -> bool {
        self.endpoint.auto_startup()
    }

    /// Sets the consumer group override.
    pub fn set_consumer_group(&mut self, consumer_group: impl Into<String>) {
        self.endpoint.set_consumer_group(consumer_group);
    }

    /// Sets the selector expression used to filter messages.
    pub fn set_selector_expression(&mut self, selector_expression: impl Into<String>) {
        self.endpoint.set_selector_expression(selector_expression);
    }

    /// Sets the message model used by this endpoint.
    pub fn set_message_model(&mut self, message_model: RocketmqMessageModel) {
        self.endpoint.set_message_model(message_model);
    }

    /// Sets whether messages should be consumed in order.
    pub fn set_orderly(&mut self, orderly: bool) {
        self.endpoint.set_orderly(orderly);
    }

    /// Sets the maximum number of messages delivered in a batch.
    pub fn set_consume_batch_size(&mut self, consume_batch_size: u32) {
        self.endpoint.set_consume_batch_size(consume_batch_size);
    }

    /// Sets whether the endpoint should start automatically.
    pub fn set_auto_startup(&mut self, auto_startup: bool) {
        self.endpoint.set_auto_startup(auto_startup);
    }
}

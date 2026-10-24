use crate::core::endpoint::RocketmqMessageModel;

/// Fluent registration contract for RocketMQ listener endpoints.
pub trait RocketmqListenerRegistration {
    /// Overrides the consumer group for the current listener.
    fn consumer_group(&mut self, consumer_group: String) -> &mut dyn RocketmqListenerRegistration;

    /// Sets the selector expression used to filter messages, for example a tag
    /// expression such as `order || refund`.
    fn selector_expression(
        &mut self,
        selector_expression: String,
    ) -> &mut dyn RocketmqListenerRegistration;

    /// Selects the RocketMQ message model used by the listener.
    fn message_model(
        &mut self,
        message_model: RocketmqMessageModel,
    ) -> &mut dyn RocketmqListenerRegistration;

    /// Enables or disables orderly consumption.
    fn orderly(&mut self, orderly: bool) -> &mut dyn RocketmqListenerRegistration;

    /// Sets the maximum number of messages delivered to the listener per batch.
    fn consume_batch_size(
        &mut self,
        consume_batch_size: u32,
    ) -> &mut dyn RocketmqListenerRegistration;

    /// Controls whether the listener should start automatically during
    /// application boot.
    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn RocketmqListenerRegistration;
}

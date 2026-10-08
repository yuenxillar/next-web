use next_web_core::async_trait;

/// A consumer of the messages of one MQTT topic filter.
///
/// A type that implements the trait is registered as `Arc<dyn TopicListener>`,
/// which is how [`MQTTAutoConfiguration`] discovers the listeners of an
/// application and builds the topic router out of them.
///
/// [`MQTTAutoConfiguration`]: crate::autoconfigure::mqtt_auto_configuration::MQTTAutoConfiguration
#[async_trait]
pub trait TopicListener
where
    Self: Send + Sync,
    Self: 'static,
{
    /// Returns the topic filter this listener handles.
    ///
    /// An exact topic (`sensors/room1`) is matched directly, while a filter
    /// that contains a wildcard (`sensors/+/temperature` or `alerts/#`) is
    /// matched against every published topic.
    fn topic(&self) -> &'static str;

    /// Handles a message that matched the topic filter of this listener.
    ///
    /// # Parameters
    ///
    /// * `topic` - The concrete topic the message was published to.
    /// * `message` - The raw payload of the message.
    async fn on_message(&self, topic: &str, message: &[u8]);
}

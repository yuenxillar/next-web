use next_web_core::{async_trait, clone_trait_object, DynClone};

/// A message interceptor of the MQTT client.
///
/// Interceptors run before a received message is dispatched to its
/// [`TopicListener`](crate::topic_listener::TopicListener), so a message can be
/// inspected and rejected before the application handles it.
#[async_trait]
pub trait MessageInterceptor
where
    Self: DynClone,
    Self: Send + Sync,
{
    /// Intercepts an incoming message and decides whether it is dispatched.
    ///
    /// # Parameters
    ///
    /// * `topic` - The topic the message was published to.
    /// * `data` - The raw payload of the message.
    ///
    /// # Returns
    ///
    /// `true` to let the message continue to the topic listeners, `false` to
    /// drop it.
    async fn message_entry(&self, topic: &str, data: &[u8]) -> bool;
}

clone_trait_object!(MessageInterceptor);

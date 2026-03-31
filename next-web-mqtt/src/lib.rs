/// MQTT support for `next-web`.
///
/// This crate contains:
/// 1. MQTT client properties and auto-registration
/// 2. topic consumers and routing
/// 3. message interception before dispatch
///
/// # Examples
///
/// ```ignore
/// use next_web_core::async_trait;
/// use next_web_mqtt::topic::topic_listener::TopicListener;
///
/// #[derive(Clone)]
/// pub struct TestTopicListener;
///
/// #[async_trait]
/// impl TopicListener for TestTopicListener {
///     fn topic(&self) -> &'static str {
///         "test/#"
///     }
///
///     async fn on_message(&self, topic: &str, message: &[u8]) {
///         println!(
///             "received topic={}, payload={:?}",
///             topic,
///             String::from_utf8_lossy(message)
///         );
///     }
/// }
/// ```
pub mod autoconfigure;
pub mod interceptor;
pub mod poll_error_handler;
pub mod service;
pub mod topic_listener;
pub mod topic_router;

pub use rumqttc::*;

pub fn generate_client_id(random: bool) -> &'static str {
    ""
}

pub mod auto_register;
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
/// use next_web_mqtt::core::topic::base_topic::BaseTopic;
///
/// #[derive(Clone)]
/// pub struct TestBaseTopic;
///
/// #[async_trait]
/// impl BaseTopic for TestBaseTopic {
///     fn topic(&self) -> &'static str {
///         "test/#"
///     }
///
///     async fn consume(&self, topic: &str, message: &[u8]) {
///         println!(
///             "received topic={}, payload={:?}",
///             topic,
///             String::from_utf8_lossy(message)
///         );
///     }
/// }
/// ```
pub mod core;
pub mod properties;
pub mod service;

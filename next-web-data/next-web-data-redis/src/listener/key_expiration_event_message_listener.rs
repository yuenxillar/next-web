use next_web_core::async_trait;

use crate::connection::default_message::DefaultMessage;

pub const KEYEVENT_EXPIRED_TOPIC: &str = "__keyevent@*__:expired";

/// Listener for Redis expired-key notifications.
///
/// To receive these callbacks, Redis must enable keyspace notifications with
/// `notify-keyspace-events Ex`.
#[async_trait]
pub trait KeyExpirationEventMessageListener
where
    Self: Send + Sync,
{
    /// Handle an expired-key notification pushed by Redis.
    ///
    /// `message` is usually the expired key name, while `pattern` is the matched
    /// Pub/Sub pattern such as `__keyevent@*__:expired`.
    async fn on_message(&self, msg: &DefaultMessage, pattern: &[u8]);
}

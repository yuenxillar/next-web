//! MQTT support for `next-web`.
//!
//! This crate contains:
//!
//! 1. MQTT client properties and auto-registration,
//! 2. topic consumers and routing,
//! 3. message interception before dispatch.
//!
//! # Features
//!
//! * `ws` - enables the plain (`ws`) and TLS-secured (`wss`) WebSocket
//!   transports, which the scheme of the broker URL selects.
//! * `trace-log` - logs the topics of received messages that no listener
//!   matches.
//!
//! # Examples
//!
//! ```ignore
//! use next_web_core::async_trait;
//! use next_web_mqtt::TopicListener;
//!
//! #[derive(Clone)]
//! pub struct TestTopicListener;
//!
//! #[async_trait]
//! impl TopicListener for TestTopicListener {
//!     fn topic(&self) -> &'static str {
//!         "test/#"
//!     }
//!
//!     async fn on_message(&self, topic: &str, message: &[u8]) {
//!         println!(
//!             "received topic={}, payload={:?}",
//!             topic,
//!             String::from_utf8_lossy(message)
//!         );
//!     }
//! }
//! ```

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::OnceLock;

pub mod autoconfigure;
pub mod interceptor;
pub mod poll_error_handler;
pub mod topic_router;

mod mqtt_client;
mod topic_listener;

pub use mqtt_client::MqttClient;
pub use topic_listener::TopicListener;

pub use rumqttc::*;

/// Returns the client identifier used when the properties configure none.
///
/// The identifier is derived from the process id and the current time, so that
/// two applications which start at the same time do not collide, and it is
/// built once and shared by every caller. When `random` is `true` a random
/// suffix is appended; the flag of the first call decides the value of the
/// shared identifier.
pub fn generate_client_id(random: bool) -> &'static str {
    static CLIENT_ID: OnceLock<String> = OnceLock::new();

    CLIENT_ID.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();

        let suffix = if random {
            format!("-{:016x}", random_seed())
        } else {
            String::new()
        };

        format!("next-web-{}-{nanos}{suffix}", std::process::id())
    })
}

/// Returns a random value taken from a random hash seed.
fn random_seed() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u8(0);

    hasher.finish()
}

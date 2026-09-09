//! Equivalent of Redisson's `LockPubSub`.

/// Redisson `LockPubSub.UNLOCK_MESSAGE`.
pub const UNLOCK_MESSAGE: i64 = 0;

/// Redisson lock channel name.
pub fn channel_name(lock_name: &str) -> String {
    super::redisson_object::prefix_name("redisson_lock__channel", lock_name)
}

//! Logical lock naming and the hash-tag rules that keep cluster routing intact.
//!
//! Redisson addresses a lock through a bare key and a `redisson_lock__channel:*`
//! channel. In cluster mode the two must land in the same slot, so the channel
//! name wraps the lock name in a hash tag. A name that already carries a tag is
//! left alone: wrapping it again would move it to a different slot.

/// The prefix of the Pub/Sub channel a lock publishes its release on.
pub const CHANNEL_PREFIX: &str = "redisson_lock__channel";

/// Prefix a name, adding a hash tag only when it does not already have one.
pub fn prefixed_name(prefix: &str, name: &str) -> String {
    if name.contains('{') {
        format!("{prefix}:{name}")
    } else {
        format!("{prefix}:{{{name}}}")
    }
}

/// The channel a lock with the given name publishes its release on.
pub fn channel_name(name: &str) -> String {
    prefixed_name(CHANNEL_PREFIX, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_preserve_an_existing_hash_tag() {
        assert_eq!(channel_name("orders"), "redisson_lock__channel:{orders}");
        assert_eq!(
            channel_name("orders:{42}"),
            "redisson_lock__channel:orders:{42}"
        );
        assert_eq!(
            prefixed_name("redisson_lock__channel", "a"),
            "redisson_lock__channel:{a}"
        );
    }
}

//! Redisson 4.7 Redis hash, Lua, channel and latch protocol.
//!
//! Wire-level constants shared by command and Pub/Sub layers. Lua scripts are
//! maintained separately in [`super::lock_scripts`].
pub(crate) const LOCK_CHANNEL_PREFIX: &str = "redisson_lock__channel";
pub(crate) const UNLOCK_LATCH_PREFIX: &str = "redisson_unlock_latch";
pub(crate) const UNLOCK_MESSAGE: i64 = 0;

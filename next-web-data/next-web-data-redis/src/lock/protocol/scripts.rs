//! The Lua scripts every lock mutation goes through, and their arguments.
//!
//! Each script is one atomic operation. Nothing outside this module writes to a
//! lock key, which is what makes reentrancy, renewal and release race-free.

/// An argument passed to a Lua script.
///
/// The scripts only ever receive integers and pre-rendered strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptArg {
    /// A millisecond lease, a return code, or the unlock message.
    Int(i64),
    /// An owner token, a channel name, or a publish command.
    Text(String),
}

impl From<i64> for ScriptArg {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<&str> for ScriptArg {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for ScriptArg {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

/// The value published on the lock channel when a lock is fully released.
///
/// Redisson publishes `0` (`LockPubSub.UNLOCK_MESSAGE`); a waiter does not
/// inspect it, so the value only has to stay stable for interoperability.
pub const UNLOCK_MESSAGE: i64 = 0;

/// One of the five lock scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockScript {
    name: &'static str,
    body: &'static str,
}

impl LockScript {
    /// Acquire one reentrant level. Returns `nil` on success, else the PTTL.
    pub const ACQUIRE: Self = Self {
        name: "acquire",
        body: include_str!("scripts/acquire.lua"),
    };

    /// Renew the lease when this owner still holds a level. Returns `1` or `0`.
    pub const RENEW: Self = Self {
        name: "renew",
        body: include_str!("scripts/renew.lua"),
    };

    /// Release one reentrant level. Returns `1`, `0`, or `-1` for a non-owner.
    pub const UNLOCK: Self = Self {
        name: "unlock",
        body: include_str!("scripts/unlock.lua"),
    };

    /// Delete the lock regardless of owner. Returns `1` or `0`.
    pub const FORCE_UNLOCK: Self = Self {
        name: "force_unlock",
        body: include_str!("scripts/force_unlock.lua"),
    };

    /// Read existence, this owner's hold count and the PTTL in one round trip.
    pub const STATUS: Self = Self {
        name: "status",
        body: include_str!("scripts/status.lua"),
    };

    /// The script name, used in errors and logs.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The Lua source.
    pub fn body(&self) -> &'static str {
        self.body
    }
}

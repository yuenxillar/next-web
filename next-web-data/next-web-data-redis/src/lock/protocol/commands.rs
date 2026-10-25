//! Request builders that bind a script's `KEYS` and `ARGV`.
//!
//! These are the only place the `KEYS`/`ARGV` order is decided, so the tests in
//! this module are the contract that the `.lua` files are written against.

use std::time::Duration;

use redis::{Cmd, FromRedisValue};

use crate::lock::error::RedisLockError;
use crate::lock::protocol::keys::channel_name;
use crate::lock::protocol::scripts::{LockScript, ScriptArg, UNLOCK_MESSAGE};

/// One script invocation, ready to hand to a backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockRequest {
    script: LockScript,
    keys: Vec<String>,
    args: Vec<ScriptArg>,
}

impl LockRequest {
    /// Start a request for `script`.
    pub fn new(script: LockScript) -> Self {
        Self {
            script,
            keys: Vec::new(),
            args: Vec::new(),
        }
    }

    /// Append a `KEYS` entry.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.keys.push(key.into());
        self
    }

    /// Append an `ARGV` entry.
    pub fn arg(mut self, arg: impl Into<ScriptArg>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// The script this request runs.
    pub fn script(&self) -> LockScript {
        self.script
    }

    /// The `KEYS` this request binds.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// The `ARGV` this request passes.
    pub fn args(&self) -> &[ScriptArg] {
        &self.args
    }

    /// Run the script on a blocking connection.
    ///
    /// The request is sent as a plain `EVAL`: the lock scripts are short, and a
    /// plain `EVAL` keeps one code path for standalone and cluster connections
    /// instead of maintaining a script cache per node.
    pub fn invoke<T: FromRedisValue>(
        &self,
        connection: &mut dyn redis::ConnectionLike,
    ) -> redis::RedisResult<T> {
        self.to_cmd().query(connection)
    }

    /// Run the script on an asynchronous connection.
    pub async fn invoke_async<T: FromRedisValue, C: redis::aio::ConnectionLike>(
        &self,
        connection: &mut C,
    ) -> redis::RedisResult<T> {
        self.to_cmd().query_async(connection).await
    }

    /// Render the request as a plain `EVAL` command.
    pub fn to_cmd(&self) -> Cmd {
        let mut command = Cmd::new();
        command
            .arg("EVAL")
            .arg(self.script.body())
            .arg(self.keys.len());
        for key in &self.keys {
            command.arg(key);
        }
        for arg in &self.args {
            match arg {
                ScriptArg::Int(value) => {
                    command.arg(value);
                }
                ScriptArg::Text(value) => {
                    command.arg(value);
                }
            }
        }
        command
    }
}

fn lease_millis(lease: Duration, field: &'static str) -> Result<i64, RedisLockError> {
    if lease.is_zero() || lease.as_millis() > i64::MAX as u128 {
        return Err(RedisLockError::InvalidDuration { field });
    }
    Ok(lease.as_millis() as i64)
}

/// Acquire one reentrant level of `name` for `owner`.
pub fn acquire(name: &str, owner: &str, lease: Duration) -> Result<LockRequest, RedisLockError> {
    let lease = lease_millis(lease, "lease")?;
    Ok(LockRequest::new(LockScript::ACQUIRE)
        .key(name)
        .arg(lease)
        .arg(owner))
}

/// Renew `owner`'s lease on `name`.
pub fn renew(name: &str, owner: &str, lease: Duration) -> Result<LockRequest, RedisLockError> {
    let lease = lease_millis(lease, "lease")?;
    Ok(LockRequest::new(LockScript::RENEW)
        .key(name)
        .arg(lease)
        .arg(owner))
}

/// Release one reentrant level of `name` held by `owner`.
///
/// `publish` is the publish command this deployment needs: `PUBLISH` for a
/// standalone server, `SPUBLISH` in cluster mode.
pub fn unlock(
    name: &str,
    owner: &str,
    lease: Duration,
    publish: &'static str,
) -> Result<LockRequest, RedisLockError> {
    let lease = lease_millis(lease, "lease")?;
    Ok(LockRequest::new(LockScript::UNLOCK)
        .key(name)
        .key(channel_name(name))
        .arg(owner)
        .arg(lease)
        .arg(publish)
        .arg(UNLOCK_MESSAGE))
}

/// Delete `name` regardless of owner and notify its channel.
pub fn force_unlock(name: &str, publish: &'static str) -> LockRequest {
    LockRequest::new(LockScript::FORCE_UNLOCK)
        .key(name)
        .key(channel_name(name))
        .arg(publish)
        .arg(UNLOCK_MESSAGE)
}

/// Read the full state of `name` as seen by `owner`.
pub fn status(name: &str, owner: &str) -> LockRequest {
    LockRequest::new(LockScript::STATUS).key(name).arg(owner)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::lock::error::RedisLockError;

    fn text(value: &str) -> ScriptArg {
        ScriptArg::Text(value.to_string())
    }

    #[test]
    fn every_builder_matches_its_lua_contract() {
        let lease = Duration::from_millis(30_000);

        let acquire = acquire("orders", "client:7", lease).unwrap();
        assert_eq!(acquire.script(), LockScript::ACQUIRE);
        assert_eq!(acquire.keys(), ["orders".to_string()].as_slice());
        assert_eq!(
            acquire.args(),
            [ScriptArg::Int(30_000), text("client:7")].as_slice()
        );

        let unlock = unlock("orders", "client:7", lease, "PUBLISH").unwrap();
        assert_eq!(
            unlock.keys(),
            [
                "orders".to_string(),
                "redisson_lock__channel:{orders}".to_string()
            ]
            .as_slice()
        );
        assert_eq!(
            unlock.args(),
            [
                text("client:7"),
                ScriptArg::Int(30_000),
                text("PUBLISH"),
                ScriptArg::Int(0)
            ]
            .as_slice()
        );

        let status = status("orders", "client:7");
        assert_eq!(status.script(), LockScript::STATUS);
        assert_eq!(status.args(), [text("client:7")].as_slice());
    }

    #[test]
    fn every_script_body_is_present() {
        for script in [
            LockScript::ACQUIRE,
            LockScript::RENEW,
            LockScript::UNLOCK,
            LockScript::FORCE_UNLOCK,
            LockScript::STATUS,
        ] {
            assert!(!script.body().is_empty(), "{} is empty", script.name());
        }
    }

    #[test]
    fn a_zero_lease_is_rejected_before_it_reaches_redis() {
        assert!(matches!(
            acquire("orders", "client:7", Duration::ZERO),
            Err(RedisLockError::InvalidDuration { field: "lease" })
        ));
    }
}

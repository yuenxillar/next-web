//! In-process backend that emulates the five lock scripts.
//!
//! Every behaviour test from the async, blocking and quorum lock layers runs
//! against this double, so `cargo test` needs no Redis. It therefore doubles as
//! executable documentation of the script contract: if the double and the Lua
//! disagree, one of them is wrong, and Task 13's live-Redis test is what catches
//! the disagreement.

use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use next_web_core::async_trait;
use redis::{ErrorKind, RedisError};
use tokio::sync::Notify;

use crate::lock::backend::{BlockingLockBackend, LockBackend};
use crate::lock::error::RedisLockError;
use crate::lock::protocol::{LockRequest, ScriptArg, ScriptReply};

/// One lock as the double remembers it.
struct MemoryLock {
    counts: HashMap<String, u32>,
    expires_at: Option<Instant>,
}

impl MemoryLock {
    fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|expires_at| expires_at <= now)
    }

    fn remaining_millis(&self, now: Instant) -> i64 {
        match self.expires_at {
            Some(expires_at) => expires_at
                .saturating_duration_since(now)
                .as_millis()
                .min(i64::MAX as u128) as i64,
            None => -1,
        }
    }
}

#[derive(Default)]
struct State {
    locks: HashMap<String, MemoryLock>,
    notifications: Vec<String>,
    renewals: HashMap<String, u32>,
    notifiers: HashMap<String, Arc<Notify>>,
    unavailable: bool,
    fail_next: bool,
    eval_delay: Option<Duration>,
    /// Bumped on every release so a blocking waiter can detect the change.
    generation: u64,
}

/// An in-memory [`LockBackend`] and [`BlockingLockBackend`] for tests.
#[derive(Default)]
pub struct MemoryLockBackend {
    state: Mutex<State>,
    changed: Condvar,
}

impl MemoryLockBackend {
    /// Create a backend other tests can share.
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Make every later call fail, as an unreachable node would.
    pub fn set_unavailable(&self, unavailable: bool) {
        self.state().unavailable = unavailable;
    }

    /// Make the next single call fail, as a transient error would.
    pub fn fail_next_eval(&self) {
        self.state().fail_next = true;
    }

    /// Make every call take `delay` before answering.
    pub fn set_eval_delay(&self, delay: Duration) {
        self.state().eval_delay = Some(delay);
    }

    /// Expire `name` as if its lease had run out.
    pub fn expire_now(&self, name: &str) {
        let mut state = self.state();
        state.locks.remove(name);
        state.generation += 1;
        drop(state);
        self.wake(name);
    }

    /// How many levels `owner` holds on `name`.
    pub fn hold_count(&self, name: &str, owner: &str) -> u32 {
        let mut state = self.state();
        expire(&mut state, name);
        state
            .locks
            .get(name)
            .and_then(|lock| lock.counts.get(owner).copied())
            .unwrap_or(0)
    }

    /// How many renewals `name` has seen.
    pub fn renew_count(&self, name: &str) -> u32 {
        self.state().renewals.get(name).copied().unwrap_or(0)
    }

    /// Whether `name` exists right now.
    pub fn exists(&self, name: &str) -> bool {
        let mut state = self.state();
        expire(&mut state, name);
        state.locks.contains_key(name)
    }

    /// The channels releases have been published on, in order.
    pub fn unlock_notifications(&self) -> Vec<String> {
        self.state().notifications.clone()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn notifier(&self, channel: &str) -> Arc<Notify> {
        let mut state = self.state();
        state
            .notifiers
            .entry(channel.to_string())
            .or_default()
            .clone()
    }

    fn wake(&self, channel: &str) {
        self.changed.notify_all();
        if let Some(notifier) = self.state().notifiers.get(channel).cloned() {
            notifier.notify_waiters();
            // `notify_one` stores a permit, so a waiter that registers just
            // after the release still returns immediately.
            notifier.notify_one();
        }
    }

    /// Emulate one script call against `state`.
    fn apply(state: &mut State, request: &LockRequest) -> Result<ScriptReply, RedisLockError> {
        if state.unavailable {
            return Err(RedisLockError::Redis(RedisError::from((
                ErrorKind::Io,
                "the memory backend is marked unavailable",
            ))));
        }
        if std::mem::take(&mut state.fail_next) {
            return Err(RedisLockError::Redis(RedisError::from((
                ErrorKind::Io,
                "the memory backend was told to fail this call",
            ))));
        }

        let name = request
            .keys()
            .first()
            .ok_or_else(|| {
                RedisLockError::InvalidConfiguration("a lock request needs a key".to_string())
            })?
            .clone();
        expire(state, &name);

        match request.script().name() {
            "acquire" => {
                let lease = int_arg(request, 0)?;
                let owner = text_arg(request, 1)?;

                if let Some(lock) = state.locks.get_mut(&name) {
                    if lock.counts.contains_key(&owner) {
                        *lock.counts.entry(owner).or_insert(0) += 1;
                        lock.expires_at =
                            Some(Instant::now() + Duration::from_millis(lease as u64));
                        return Ok(ScriptReply::Nil);
                    }
                    return Ok(ScriptReply::Int(lock.remaining_millis(Instant::now())));
                }

                let mut counts = HashMap::new();
                counts.insert(owner, 1);
                state.locks.insert(
                    name,
                    MemoryLock {
                        counts,
                        expires_at: Some(Instant::now() + Duration::from_millis(lease as u64)),
                    },
                );
                Ok(ScriptReply::Nil)
            }
            "renew" => {
                let lease = int_arg(request, 0)?;
                let owner = text_arg(request, 1)?;

                match state.locks.get_mut(&name) {
                    Some(lock) if lock.counts.contains_key(&owner) => {
                        lock.expires_at =
                            Some(Instant::now() + Duration::from_millis(lease as u64));
                        *state.renewals.entry(name).or_insert(0) += 1;
                        Ok(ScriptReply::Int(1))
                    }
                    _ => Ok(ScriptReply::Int(0)),
                }
            }
            "unlock" => {
                let owner = text_arg(request, 0)?;
                let lease = int_arg(request, 1)?;

                let Some(lock) = state.locks.get_mut(&name) else {
                    return Ok(ScriptReply::Int(-1));
                };
                if !lock.counts.contains_key(&owner) {
                    return Ok(ScriptReply::Int(-1));
                }

                let level = lock.counts.get_mut(&owner).map(|count| {
                    *count -= 1;
                    *count
                });
                if level.is_some_and(|level| level > 0) {
                    lock.expires_at = Some(Instant::now() + Duration::from_millis(lease as u64));
                    return Ok(ScriptReply::Int(0));
                }

                let channel = channel_of(request);
                state.locks.remove(&name);
                state.notifications.push(channel.clone());
                state.generation += 1;
                if let Some(notifier) = state.notifiers.get(&channel) {
                    notifier.notify_waiters();
                    notifier.notify_one();
                }
                Ok(ScriptReply::Int(1))
            }
            "force_unlock" => {
                if state.locks.remove(&name).is_none() {
                    return Ok(ScriptReply::Int(0));
                }
                let channel = channel_of(request);
                state.notifications.push(channel.clone());
                state.generation += 1;
                if let Some(notifier) = state.notifiers.get(&channel) {
                    notifier.notify_waiters();
                    notifier.notify_one();
                }
                Ok(ScriptReply::Int(1))
            }
            "status" => {
                let owner = text_arg(request, 0)?;
                match state.locks.get(&name) {
                    Some(lock) => Ok(ScriptReply::Ints(vec![
                        1,
                        i64::from(lock.counts.get(&owner).copied().unwrap_or(0)),
                        lock.remaining_millis(Instant::now()),
                    ])),
                    None => Ok(ScriptReply::Ints(vec![0, 0, -2])),
                }
            }
            other => Err(RedisLockError::InvalidConfiguration(format!(
                "the memory backend does not emulate `{other}`"
            ))),
        }
    }

    fn eval_now(&self, request: &LockRequest) -> Result<ScriptReply, RedisLockError> {
        let mut state = self.state();
        Self::apply(&mut state, request)
    }
}

#[async_trait]
impl LockBackend for MemoryLockBackend {
    async fn eval(
        &self,
        request: &LockRequest,
        timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError> {
        let delay = self.state().eval_delay;
        if let Some(delay) = delay {
            // The double enforces the command timeout the way a real backend
            // does, so a hung node can be tested without one.
            tokio::time::timeout(timeout, tokio::time::sleep(delay))
                .await
                .map_err(|_| RedisLockError::CommandTimeout {
                    script: request.script().name(),
                })?;
        }

        self.eval_now(request)
    }

    async fn wait_for_unlock(
        &self,
        name: &str,
        channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError> {
        // Register interest before checking existence: a release between the
        // check and the wait would otherwise be lost.
        let notifier = self.notifier(channel);
        let notified = notifier.notified_owned();

        if !self.exists(name) {
            return Ok(());
        }

        let _ = tokio::time::timeout(timeout, notified).await;

        Ok(())
    }
}

impl BlockingLockBackend for MemoryLockBackend {
    fn eval(
        &self,
        request: &LockRequest,
        timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError> {
        let delay = self.state().eval_delay;
        if let Some(delay) = delay {
            let started = Instant::now();
            std::thread::sleep(delay);
            if started.elapsed() > timeout {
                return Err(RedisLockError::CommandTimeout {
                    script: request.script().name(),
                });
            }
        }

        self.eval_now(request)
    }

    fn wait_for_unlock(
        &self,
        name: &str,
        _channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError> {
        let deadline = Instant::now().checked_add(timeout);
        let mut state = self.state();

        if !state.locks.contains_key(name) {
            return Ok(());
        }

        let generation = state.generation;
        while state.generation == generation && state.locks.contains_key(name) {
            let remaining = match deadline {
                Some(deadline) => deadline.saturating_duration_since(Instant::now()),
                None => Duration::ZERO,
            };
            if remaining.is_zero() {
                break;
            }
            let (guard, _) = self
                .changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = guard;
        }

        Ok(())
    }
}

fn expire(state: &mut State, name: &str) {
    let expired = state
        .locks
        .get(name)
        .is_some_and(|lock| lock.is_expired(Instant::now()));
    if expired {
        state.locks.remove(name);
        state.generation += 1;
    }
}

fn channel_of(request: &LockRequest) -> String {
    request
        .keys()
        .get(1)
        .cloned()
        .unwrap_or_else(|| format!("redisson_lock__channel:{{{}}}", request.keys()[0]))
}

fn int_arg(request: &LockRequest, index: usize) -> Result<i64, RedisLockError> {
    match request.args().get(index) {
        Some(ScriptArg::Int(value)) => Ok(*value),
        _ => Err(RedisLockError::UnexpectedReply(format!(
            "`{}` is missing integer argument {index}",
            request.script().name()
        ))),
    }
}

fn text_arg(request: &LockRequest, index: usize) -> Result<String, RedisLockError> {
    match request.args().get(index) {
        Some(ScriptArg::Text(value)) => Ok(value.clone()),
        _ => Err(RedisLockError::UnexpectedReply(format!(
            "`{}` is missing text argument {index}",
            request.script().name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::lock::backend::{BlockingLockBackend, LockBackend};
    use crate::lock::protocol::{
        ScriptReply,
        commands::{acquire, renew, unlock},
    };

    #[tokio::test]
    async fn the_double_emulates_the_acquire_and_unlock_contract() {
        let backend = MemoryLockBackend::new_shared();
        let asynchronous: &dyn LockBackend = backend.as_ref();

        let first = acquire("orders", "c:1", Duration::from_secs(30)).unwrap();
        assert_eq!(
            asynchronous
                .eval(&first, Duration::from_secs(1))
                .await
                .unwrap(),
            ScriptReply::Nil
        );

        let contended = acquire("orders", "c:2", Duration::from_secs(30)).unwrap();
        assert!(matches!(
            asynchronous
                .eval(&contended, Duration::from_secs(1))
                .await
                .unwrap(),
            ScriptReply::Int(ttl) if ttl > 0
        ));

        let release = unlock("orders", "c:1", Duration::from_secs(30), "PUBLISH").unwrap();
        assert_eq!(
            asynchronous
                .eval(&release, Duration::from_secs(1))
                .await
                .unwrap()
                .int(),
            Some(1)
        );
        assert_eq!(
            backend.unlock_notifications(),
            vec!["redisson_lock__channel:{orders}".to_string()]
        );
        assert!(!backend.exists("orders"));
    }

    #[tokio::test]
    async fn the_double_rejects_a_foreign_unlock_without_touching_state() {
        let backend = MemoryLockBackend::new_shared();
        let asynchronous: &dyn LockBackend = backend.as_ref();

        asynchronous
            .eval(
                &acquire("orders", "c:1", Duration::from_secs(30)).unwrap(),
                Duration::from_secs(1),
            )
            .await
            .unwrap();

        let foreign = unlock("orders", "c:2", Duration::from_secs(30), "PUBLISH").unwrap();
        assert_eq!(
            asynchronous
                .eval(&foreign, Duration::from_secs(1))
                .await
                .unwrap()
                .int(),
            Some(-1)
        );
        assert_eq!(backend.hold_count("orders", "c:1"), 1);
        assert!(backend.exists("orders"));
    }

    #[test]
    fn the_double_counts_renewals_and_honours_expiry() {
        let backend = MemoryLockBackend::new_shared();
        let blocking: &dyn BlockingLockBackend = backend.as_ref();

        blocking
            .eval(
                &acquire("orders", "c:1", Duration::from_millis(60)).unwrap(),
                Duration::from_secs(1),
            )
            .unwrap();
        blocking
            .eval(
                &renew("orders", "c:1", Duration::from_millis(60)).unwrap(),
                Duration::from_secs(1),
            )
            .unwrap();
        assert_eq!(backend.renew_count("orders"), 1);

        backend.expire_now("orders");
        assert!(!backend.exists("orders"));
    }
}

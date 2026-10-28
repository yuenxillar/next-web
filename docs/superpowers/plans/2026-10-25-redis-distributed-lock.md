# Redis Distributed Lock Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single-file `next-web-data-redis` lock subsystem with a module-per-responsibility implementation that offers an async API and a native blocking API, both driven by one Lua-only atomic protocol and a `RedisTemplate`-implemented backend trait.

**Architecture:** A `LockBackend` trait (async) and a `BlockingLockBackend` trait (sync) expose exactly two primitives — run one Lua script atomically, and wait for an unlock notification. `RedisTemplate` implements both; a cluster implementation and an in-crate in-memory test double implement both as well. `RedisLock`, `BlockingRedisLock` and `RedLock` are thin state machines on top of those primitives: the only thing that mutates lock state is a script call, so reentrancy, lease renewal and release are atomic by construction. The blocking API uses the `redis` crate's own synchronous client per the project owner's instruction, never `block_on` over the async path.

**Tech Stack:** Rust 1.98.1, edition 2024, `redis` 1.8.0 (`tokio-comp`, `cluster-async`), `tokio` (rt, sync, time), `futures`, `thiserror` 2, `uuid` 1 (v4), `tracing`. No new dependencies.

**Spec:** Inline — this plan carries the API surface, wire protocol and module layout agreed in this session (no separate design doc; the user asked for the plan directly).

## Global Constraints

- Crate: `next-web-data/next-web-data-redis`. Every item in this plan is compiled only with `--features distributed-lock`; the crate's `default` feature already enables it.
- Toolchain floor `rust-version = "1.98.1"`, `edition = "2024"`.
- Workspace lints that gate this work: `unsafe_code = "forbid"`, `missing_docs = "warn"`, `unreachable_pub = "warn"`. Every public item gets an English `///` doc comment, including enum variants and struct fields.
- No `unwrap()`/`expect()`/`panic!` in library code. Errors go through `Result` and `#[from] RedisError` conversion. `unwrap` is allowed in `#[cfg(test)]` code only.
- No `block_on`, no `futures::executor::block_on`, no `Handle::block_on`, no `block_in_place` anywhere. The blocking API calls the synchronous `redis` client directly; the async API never calls the blocking client.
- Lock state is mutated only inside a Lua script. No `HSET`/`HDEL`/`DEL`/`PEXPIRE` on a lock key from Rust.
- Wire format stays Redisson 4.7-compatible so a Java Redisson client and this crate exclude each other: lock key is the plain name, value is a hash `{ "<client-id>:<owner-seq>": hold-count }`, channel is `redisson_lock__channel:{<name>}`, unlock message is `0`.
- Key names must keep a hash tag (`{name}`) so cluster mode routes the lock key and its channel to the same slot. A name that already contains `{` is not re-wrapped.
- No new crate dependencies. `tokio` gains the `sync` and `time` features in this crate's `Cargo.toml` only.
- Commit style in this repo: `refactor(redis-lock): ...`, `feat(redis-lock): ...`, `test(redis-lock): ...`.

## Review Focus

- **Two owners must never hold the same lock at the same time.** A second `get_lock` (different owner token) must fail while the first is held, and must succeed after release. Pinned by `two_owners_are_mutually_exclusive` (Task 8) and `blocking_two_owners_are_mutually_exclusive` (Task 9).
- **Releasing a lock you do not own must change nothing.** A foreign `unlock` must return `NotOwner` and leave the hash, its TTL and the channel untouched. Pinned by the `-1` branch of `unlock.lua` (Task 3), by `the_double_rejects_a_foreign_unlock_without_touching_state` (Task 7), and by `force_unlock_releases_a_lock_the_caller_does_not_own` (Task 8).
- **Watchdog lifetime must be tied to the guard.** Renewal must stop when the guard is unlocked *and* when it is dropped, and a guard whose lease was lost must report `WatchdogLost` instead of pretending to release. Pinned by `watchdog_renews_until_unlock` and `lost_lease_is_reported_on_unlock` (Task 8) and `a_dropped_blocking_guard_stops_renewing` (Task 9).
- **Reentrant acquisition depth is per owner token.** Clones of one handle re-enter; unlocking at depth 2 must keep the lock alive and only the depth-1 unlock must publish. Pinned by `second_lock_on_the_same_handle_is_reentrant` (Task 8) and its blocking twin (Task 9).
- **A hung node must fail in bounded time, not hang the caller.** Every script call carries `command_timeout` and the backend enforces it, and `try_lock_for` never exceeds its `wait` budget. Pinned by `eval_timeout_surfaces_as_error` and `try_lock_for_respects_the_wait_budget` (Task 8).

---

## API Surface (decided here, referenced by every task)

```rust
// crate root re-exports once Task 12 lands
pub use lock::{
    BlockingLockBackend, BlockingRedisLock, BlockingRedisLockGuard, BlockingRedLock,
    BlockingRedLockGuard, LockBackend, LockRequest, LockScript, RedLock, RedLockGuard,
    RedisLock, RedisLockConfig, RedisLockError, RedisLockGuard, RedisLockService,
    ScriptArg, ScriptReply, UnlockOutcome,
};
```

| Type | Purpose | Naming rationale |
|---|---|---|
| `RedisLock` | async reentrant lock on one deployment | matches `RedisTemplate`; no `Redisson*` prefix |
| `RedisLockGuard` | async guard, `async fn unlock(self)` | returned by `lock()`/`try_lock*()` |
| `BlockingRedisLock` | same semantics, synchronous methods | "blocking" says what it does to the thread |
| `BlockingRedisLockGuard` | sync guard, `fn unlock(self)` | |
| `RedLock` / `RedLockGuard` | async quorum over ≥3 independent deployments | established term in the Redis community |
| `BlockingRedLock` / `BlockingRedLockGuard` | blocking quorum twin | same shape as `BlockingRedisLock` |
| `RedisLockService` | factory: `get_lock`, `get_blocking_lock`, `get_red_lock`, `get_blocking_red_lock` | replaces `RedisDistributedLockService` |
| `RedisLockError` / `UnlockOutcome` | replaces `LockError` / `UnlockResult` | crate-qualified so it cannot collide with a future `std` item |

Method sets (async names are mirrored 1:1 on the blocking type without `async`):

```rust
impl RedisLock {
    pub fn name(&self) -> &str;
    pub fn owner_id(&self) -> &str;
    pub async fn lock(&self) -> Result<RedisLockGuard, RedisLockError>;
    pub async fn lock_with_lease(&self, lease: Duration) -> Result<RedisLockGuard, RedisLockError>;
    pub async fn try_lock(&self) -> Result<Option<RedisLockGuard>, RedisLockError>;
    pub async fn try_lock_for(&self, wait: Duration) -> Result<Option<RedisLockGuard>, RedisLockError>;
    pub async fn try_lock_for_with_lease(&self, wait: Duration, lease: Duration) -> Result<Option<RedisLockGuard>, RedisLockError>;
    pub async fn is_locked(&self) -> Result<bool, RedisLockError>;
    pub async fn is_held_by_owner(&self) -> Result<bool, RedisLockError>;
    pub async fn hold_count(&self) -> Result<u32, RedisLockError>;
    pub async fn remaining_lease(&self) -> Result<Option<Duration>, RedisLockError>;
    pub async fn force_unlock(&self) -> Result<bool, RedisLockError>;
    pub async fn with_lock<T, E, F, Fut>(&self, operation: F) -> Result<Result<T, E>, RedisLockError>
    where F: FnOnce() -> Fut, Fut: Future<Output = Result<T, E>>;
}
```

Semantics that are not inferable and are therefore fixed here:

- `lock()` waits indefinitely with the watchdog lease (`config.watchdog_timeout`, renewed every lease/3).
- `lock_with_lease(lease)` waits indefinitely with a fixed lease and **no** watchdog.
- `try_lock()` attempts once, no wait, watchdog lease.
- `try_lock_for(wait)` retries until `wait` elapses, watchdog lease.
- `try_lock_for_with_lease(wait, lease)` retries until `wait` elapses, fixed lease, no watchdog.
- Reentrancy is **per owner token**: `get_lock(name)` mints a token; clones of a handle share it; a second `get_lock(name)` call mints a different token and therefore contends.
- `with_lock` always releases before returning, including when the operation returns `Err`; a failed release is returned as the outer `Err` only when the operation itself succeeded.

---

## Wire Protocol (fixed here, implemented by Task 3)

Five scripts, each one atomic operation. `KEYS`/`ARGV` order is the contract between `protocol::commands` and `protocol::scripts`; Task 3's unit tests pin it.

`scripts/acquire.lua` — `KEYS[1]`=lock, `ARGV[1]`=lease_ms, `ARGV[2]`=owner. Returns `nil` when acquired, else the remaining PTTL in ms.

```lua
if (redis.call('exists', KEYS[1]) == 0) or (redis.call('hexists', KEYS[1], ARGV[2]) == 1) then
  redis.call('hincrby', KEYS[1], ARGV[2], 1)
  redis.call('pexpire', KEYS[1], ARGV[1])
  return nil
end
return redis.call('pttl', KEYS[1])
```

`scripts/renew.lua` — `KEYS[1]`=lock, `ARGV[1]`=lease_ms, `ARGV[2]`=owner. Returns `1` renewed, `0` owner no longer holds it.

```lua
if (redis.call('hexists', KEYS[1], ARGV[2]) == 1) then
  redis.call('pexpire', KEYS[1], ARGV[1])
  return 1
end
return 0
```

`scripts/unlock.lua` — `KEYS[1]`=lock, `KEYS[2]`=channel, `ARGV[1]`=owner, `ARGV[2]`=lease_ms, `ARGV[3]`=publish command (`PUBLISH` or `SPUBLISH`), `ARGV[4]`=unlock message. Returns `-1` not owner, `0` reentrant level released, `1` fully released (lock deleted and notified).

```lua
if (redis.call('hexists', KEYS[1], ARGV[1]) == 0) then
  return -1
end
local counter = redis.call('hincrby', KEYS[1], ARGV[1], -1)
if (counter > 0) then
  redis.call('pexpire', KEYS[1], ARGV[2])
  return 0
end
redis.call('del', KEYS[1])
local published = redis.pcall(ARGV[3], KEYS[2], ARGV[4])
if (type(published) == 'table' and published.err) then
  redis.call('publish', KEYS[2], ARGV[4])
end
return 1
```

`scripts/force_unlock.lua` — `KEYS[1]`=lock, `KEYS[2]`=channel, `ARGV[1]`=publish command, `ARGV[2]`=unlock message. Returns `1` deleted, `0` nothing to delete.

```lua
if (redis.call('del', KEYS[1]) == 0) then
  return 0
end
local published = redis.pcall(ARGV[1], KEYS[2], ARGV[2])
if (type(published) == 'table' and published.err) then
  redis.call('publish', KEYS[2], ARGV[2])
end
return 1
```

`scripts/status.lua` — `KEYS[1]`=lock, `ARGV[1]`=owner. Returns a 3-element array `{exists, hold_count, pttl_ms}`; `pttl_ms` is `-2` when the key is gone.

```lua
if (redis.call('exists', KEYS[1]) == 0) then
  return {0, 0, -2}
end
local count = redis.call('hget', KEYS[1], ARGV[1])
return {1, tonumber(count) or 0, redis.call('pttl', KEYS[1])}
```

Key naming (`protocol::keys`):

- `lock_key(name)` → `name` unchanged (Redisson writes the bare name).
- `channel_name(name)` → `redisson_lock__channel:<name>` when `name` contains `{`, else `redisson_lock__channel:{<name>}`.
- `prefixed_name(prefix, name)` → `<prefix>:<name>` when `name` contains `{`, else `<prefix>:{<name>}`.

---

## File Structure

```
next-web-data/next-web-data-redis/
├── Cargo.toml                                  modify: tokio features
├── README.md                                   modify: document the new API
├── src/
│   ├── lib.rs                                  modify: re-exports (Task 12)
│   ├── core/
│   │   └── redis_template.rs                   modify: impl both backend traits (Task 5)
│   ├── autoconfigure/
│   │   ├── redis_properties.rs                 modify: lock accessors (Task 12)
│   │   └── redis_auto_configuration.rs         modify: register the service (Task 12)
│   └── lock/
│       ├── mod.rs                              rewrite: module tree + re-exports
│       ├── error.rs                            rewrite: RedisLockError, UnlockOutcome
│       ├── config.rs                           rewrite: RedisLockConfig
│       ├── ownership.rs                        create: owner tokens
│       ├── lease.rs                            create: lease/renewal math, lost flag
│       ├── guard.rs                            create: GuardCore shared by both guards
│       ├── backend/
│       │   ├── mod.rs                          create: LockBackend, BlockingLockBackend
│       │   ├── cluster.rs                      create: cluster impls (Task 6)
│       │   └── memory.rs                       create: #[cfg(test)] test double (Task 7)
│       ├── protocol/
│       │   ├── mod.rs                          create
│       │   ├── keys.rs                         create: naming
│       │   ├── reply.rs                        create: ScriptReply parsing
│       │   ├── commands.rs                     create: LockRequest builders
│       │   ├── scripts.rs                      create: LockScript constants
│       │   └── scripts/
│       │       ├── acquire.lua                 create
│       │       ├── renew.lua                   create
│       │       ├── unlock.lua                  create
│       │       ├── force_unlock.lua            create
│       │       └── status.lua                  create
│       ├── async_lock/
│       │   ├── mod.rs                          create
│       │   ├── lock.rs                         create: RedisLock (Task 8)
│       │   ├── guard.rs                        create: RedisLockGuard (Task 8)
│       │   ├── watchdog.rs                     create: tokio renewal task (Task 8)
│       │   └── waiter.rs                       create: retry delay policy (Task 8)
│       ├── blocking/
│       │   ├── mod.rs                          create
│       │   ├── lock.rs                         create: BlockingRedisLock (Task 9)
│       │   ├── guard.rs                        create: BlockingRedisLockGuard (Task 9)
│       │   └── watchdog.rs                     create: std::thread renewal (Task 9)
│       ├── redlock/
│       │   ├── mod.rs                          create
│       │   └── lock.rs                         create: RedLock, BlockingRedLock (Task 10)
│       ├── service.rs                          create: RedisLockService (Task 11)
│       ├── test_support.rs                     create: #[cfg(test)] shared helpers (Task 8)
│       └── (deleted in Task 12) redisson_lock.rs, redisson_base_lock.rs,
│           redisson_expirable.rs, redisson_multi_lock.rs, redisson_object.rs,
│           redisson_red_lock.rs, rlock.rs, redlock.rs, lock_pub_sub.rs,
│           lock_renewal_scheduler.rs, lock_scripts.rs, backend.rs, protocol.rs,
│           scripts/*.lua (old copies)
└── tests/
    └── distributed_lock.rs                     rewrite (Tasks 12, 13)
```

Rationale for the split: `protocol` owns everything that is bytes-on-the-wire, `backend` owns everything that is IO, `async_lock`/`blocking` own the two state machines, `redlock` owns composition, `service` owns construction. No file exceeds ~250 lines.

---

### Task 1: Green baseline

The tree currently fails to compile for two unrelated reasons left over from an interrupted move of `src/service/lock` to `src/lock`. Fix both so every later failure is attributable.

**Files:**
- Modify: `next-web-data/next-web-data-redis/src/autoconfigure/redis_properties.rs:8`
- Modify: `next-web-data/next-web-data-redis/src/lock/redisson_lock.rs:174`

**Interfaces:**
- Consumes: nothing.
- Produces: a crate where `cargo check -p next-web-data-redis --features distributed-lock` is clean.

- [ ] **Step 1: Confirm the failure**

Run: `cargo check -p next-web-data-redis --features distributed-lock --message-format short`
Expected: two errors — `cannot find 'service' in 'crate'` at `redis_properties.rs` and ``the trait bound `Atomic<u32>: Clone` is not satisfied`` at `redisson_lock.rs`.

- [ ] **Step 2: Point the properties import at the new module path**

In `redis_properties.rs` replace `use crate::service::lock::{RedisLockConfig, RedisLockMode};` with `use crate::lock::{RedisLockConfig, RedisLockMode};`.

- [ ] **Step 3: Drop the copy derive from `LockTarget`**

`LockTarget` is only ever used behind `Arc` (`self.clone()` clones the `Arc`), so `#[derive(Clone)]` on line 174 is both unused and illegal because of `local_hold_count: AtomicU32`.

- [ ] **Step 4: Verify**

Run: `cargo check -p next-web-data-redis --features distributed-lock`
Expected: `Finished` with only the two pre-existing warnings (`unused mut` at `redisson_lock.rs:121`, `unused variable ctx` at `redis_auto_configuration.rs:113`).

- [ ] **Step 5: Verify the no-default-features build too**

Run: `cargo check -p next-web-data-redis --no-default-features`
Expected: `Finished`.

- [ ] **Step 6: Commit**

```bash
git add next-web-data/next-web-data-redis/src/autoconfigure/redis_properties.rs next-web-data/next-web-data-redis/src/lock/redisson_lock.rs
git commit -m "fix(redis-lock): repair the module path and the lock target clone after the move"
```

---

### Task 2: Errors, configuration, script replies

**Files:**
- Rewrite: `next-web-data/next-web-data-redis/src/lock/error.rs`
- Rewrite: `next-web-data/next-web-data-redis/src/lock/config.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/protocol/mod.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/protocol/reply.rs`
- Delete: `next-web-data/next-web-data-redis/src/lock/protocol.rs` (a file and a directory of the same name cannot coexist)
- Modify: `next-web-data/next-web-data-redis/src/lock/mod.rs` (declare `pub mod protocol;`)

**Interfaces:**
- Consumes: `redis::Value`, `redis::RedisError`.
- Produces:
  - `#[derive(Debug, thiserror::Error)] pub enum RedisLockError { Redis(#[from] redis::RedisError), InvalidConfiguration(String), InvalidDuration { field: &'static str }, NotOwner, WatchdogLost, QuorumUnavailable { required: usize, acquired: usize }, CommandTimeout { script: &'static str }, UnexpectedReply(String), PartiallyReleased { failures: usize } }`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum UnlockOutcome { Reentrant, Released }`
  - `#[derive(Debug, Clone)] pub struct RedisLockConfig { pub watchdog_timeout: Duration, pub retry_interval: Duration, pub command_timeout: Duration }` with `Default` = 30 s / 100 ms / 3 s and `pub fn validate(&self) -> Result<(), RedisLockError>`
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum ScriptReply { Nil, Int(i64), Ints(Vec<i64>) }` with `pub fn parse(value: redis::Value) -> Result<ScriptReply, RedisLockError>`, `pub fn int(&self) -> Option<i64>`, `pub fn ints(&self) -> Option<&[i64]>`, `pub fn is_nil(&self) -> bool`

- [ ] **Step 1: Write the failing tests**

In `config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_redisson_watchdog() {
        let config = RedisLockConfig::default();
        assert_eq!(config.watchdog_timeout, Duration::from_secs(30));
        assert_eq!(config.retry_interval, Duration::from_millis(100));
        assert_eq!(config.command_timeout, Duration::from_secs(3));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validation_names_the_offending_field() {
        let zero_retry = RedisLockConfig { retry_interval: Duration::ZERO, ..Default::default() };
        assert!(matches!(
            zero_retry.validate(),
            Err(RedisLockError::InvalidDuration { field: "retry_interval" })
        ));
        let too_long = RedisLockConfig { command_timeout: Duration::from_millis(u64::MAX).saturating_add(Duration::from_secs(1)), ..Default::default() };
        assert!(matches!(
            too_long.validate(),
            Err(RedisLockError::InvalidDuration { field: "command_timeout" })
        ));
    }
}
```

In `protocol/reply.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use redis::Value;

    #[test]
    fn nil_means_acquired() {
        assert_eq!(ScriptReply::parse(Value::Nil).unwrap(), ScriptReply::Nil);
    }

    #[test]
    fn integers_and_arrays_round_trip() {
        assert_eq!(ScriptReply::parse(Value::Int(-1)).unwrap().int(), Some(-1));
        let reply = ScriptReply::parse(Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3000)])).unwrap();
        assert_eq!(reply.ints(), Some(&[1, 2, 3000][..]));
    }

    #[test]
    fn a_bulk_string_is_not_a_script_reply() {
        let error = ScriptReply::parse(Value::BulkString(b"nope".to_vec())).unwrap_err();
        assert!(matches!(error, RedisLockError::UnexpectedReply(_)));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::config lock::protocol::reply`
Expected: compile errors — the types do not exist yet.

- [ ] **Step 3: Implement the three files**

`UnlockOutcome`, `RedisLockError`, `RedisLockConfig::validate` (zero or `as_millis() > u64::MAX as u128`), and `ScriptReply::parse` over `Value::Nil | Value::Int | Value::Array(Vec<Value::Int>)`, mapping anything else to `RedisLockError::UnexpectedReply(format!("unexpected reply: {other:?}"))`. `UnlockOutcome` is consumed by Tasks 8–10 as the return of a successful release.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::config lock::protocol::reply`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/error.rs next-web-data/next-web-data-redis/src/lock/config.rs next-web-data/next-web-data-redis/src/lock/protocol next-web-data/next-web-data-redis/src/lock/mod.rs
git commit -m "feat(redis-lock): add lock errors, configuration and script reply parsing"
```

---

### Task 3: Protocol — keys, scripts, request builders

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/protocol/keys.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/protocol/scripts.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/protocol/commands.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/protocol/scripts/{acquire,renew,unlock,force_unlock,status}.lua` (bodies verbatim from "Wire Protocol" above)
- Modify: `next-web-data/next-web-data-redis/src/lock/protocol/mod.rs` (declare `keys`, `scripts`, `commands`)

**Interfaces:**
- Consumes: `RedisLockError`, `ScriptReply`.
- Produces:
  - `pub fn prefixed_name(prefix: &str, name: &str) -> String`, `pub fn channel_name(name: &str) -> String`
  - `pub const UNLOCK_MESSAGE: i64 = 0`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct LockScript` with `pub const ACQUIRE/RENEW/UNLOCK/FORCE_UNLOCK/STATUS: LockScript`, `pub fn name(&self) -> &'static str`, `pub fn body(&self) -> &'static str`
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum ScriptArg { Int(i64), Text(String) }` with `From<i64>`, `From<&str>`, `From<String>`
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct LockRequest` with `pub fn script(&self) -> LockScript`, `pub fn keys(&self) -> &[String]`, `pub fn args(&self) -> &[ScriptArg]`, `pub fn invoke<T: FromRedisValue>(&self, con: &mut dyn redis::ConnectionLike) -> redis::RedisResult<T>`, `pub async fn invoke_async<T: FromRedisValue, C: redis::aio::ConnectionLike>(&self, con: &mut C) -> redis::RedisResult<T>`
  - Builders: `pub fn acquire(name: &str, owner: &str, lease: Duration) -> Result<LockRequest, RedisLockError>`, `pub fn renew(name: &str, owner: &str, lease: Duration) -> Result<LockRequest, RedisLockError>`, `pub fn unlock(name: &str, owner: &str, lease: Duration, publish: &'static str) -> Result<LockRequest, RedisLockError>`, `pub fn force_unlock(name: &str, publish: &'static str) -> LockRequest`, `pub fn status(name: &str, owner: &str) -> LockRequest`

- [ ] **Step 1: Write the failing tests**

In `keys.rs`:

```rust
#[test]
fn names_preserve_an_existing_hash_tag() {
    assert_eq!(channel_name("orders"), "redisson_lock__channel:{orders}");
    assert_eq!(channel_name("orders:{42}"), "redisson_lock__channel:orders:{42}");
    assert_eq!(prefixed_name("redisson_lock__channel", "a"), "redisson_lock__channel:{a}");
}
```

In `commands.rs`:

```rust
fn text(value: &str) -> ScriptArg {
    ScriptArg::Text(value.to_string())
}

#[test]
fn every_builder_matches_its_lua_contract() {
    let lease = Duration::from_millis(30_000);
    let acquire = acquire("orders", "client:7", lease).unwrap();
    assert_eq!(acquire.script(), LockScript::ACQUIRE);
    assert_eq!(acquire.keys(), ["orders".to_string()].as_slice());
    assert_eq!(acquire.args(), [ScriptArg::Int(30_000), text("client:7")].as_slice());

    let unlock = unlock("orders", "client:7", lease, "PUBLISH").unwrap();
    assert_eq!(
        unlock.keys(),
        ["orders".to_string(), "redisson_lock__channel:{orders}".to_string()].as_slice()
    );
    assert_eq!(
        unlock.args(),
        [text("client:7"), ScriptArg::Int(30_000), text("PUBLISH"), ScriptArg::Int(0)].as_slice()
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
```

Every `LockScript` body must also be asserted non-empty in one test (`assert!(!LockScript::ACQUIRE.body().is_empty())` for all five) so a script-path typo cannot pass review.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::protocol::`
Expected: compile errors — modules missing.

- [ ] **Step 3: Implement**

`LockScript` is a `{ name, body }` struct whose constants use `include_str!("scripts/acquire.lua")` etc. `LockRequest::invoke` builds `redis::Script::new(self.script.body())`, chains `.key()` for each key and `.arg()` for each arg (Int → `i64`, Text → `&str`), then calls `Script::invoke` / `invoke_async`. The builders delegate to `script_arg_millis(lease, "lease")` so the zero/overflow rule lives in one place.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::protocol::`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/protocol
git commit -m "feat(redis-lock): add the key naming, the five Lua scripts and the request builders"
```

---

### Task 4: Backend traits, ownership tokens, lease math

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/backend/mod.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/ownership.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/lease.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/guard.rs`
- Delete: `next-web-data/next-web-data-redis/src/lock/backend.rs`
- Modify: `next-web-data/next-web-data-redis/src/lock/mod.rs`

**Interfaces:**
- Consumes: `LockRequest`, `ScriptReply`, `RedisLockError`.
- Produces:
  - `#[async_trait] pub trait LockBackend: Send + Sync + 'static { async fn eval(&self, request: &LockRequest, timeout: Duration) -> Result<ScriptReply, RedisLockError>; async fn wait_for_unlock(&self, name: &str, channel: &str, timeout: Duration) -> Result<(), RedisLockError>; fn publish_command(&self) -> &'static str { "PUBLISH" } }`
  - `pub trait BlockingLockBackend: Send + Sync + 'static { fn eval(&self, request: &LockRequest, timeout: Duration) -> Result<ScriptReply, RedisLockError>; fn wait_for_unlock(&self, name: &str, channel: &str, timeout: Duration) -> Result<(), RedisLockError>; fn publish_command(&self) -> &'static str { "PUBLISH" } }`
  - `#[derive(Debug, Clone, Default)] pub struct LockOwner` with `pub fn next_token(&self) -> OwnerToken`; `#[derive(Debug, Clone, PartialEq, Eq)] pub struct OwnerToken(Arc<str>)` with `pub fn as_str(&self) -> &str`
  - `#[derive(Debug, Clone)] pub struct Lease { pub duration: Duration, pub watchdog: bool }` with `pub fn renewal_interval(&self) -> Duration` (`max(duration / 3, 1 ms)`)
  - `pub(crate) struct NodeLease { pub index: usize, pub lease: Lease, pub lost: Arc<AtomicBool> }` with `pub fn is_lost(&self) -> bool`
  - `pub(crate) struct GuardCore { pub name: Arc<str>, pub owner: OwnerToken, pub quorum: usize, pub leases: Vec<NodeLease> }` with `pub fn is_lost(&self) -> bool` (count of non-lost leases < quorum)

- [ ] **Step 1: Write the failing tests**

In `ownership.rs`:

```rust
#[test]
fn every_token_is_distinct_and_stable() {
    let owner = LockOwner::default();
    let first = owner.next_token();
    let second = owner.next_token();
    assert_ne!(first, second);
    assert_eq!(first.as_str(), first.clone().as_str());
    assert!(first.as_str().contains(':'));
}
```

In `lease.rs`:

```rust
#[test]
fn renewal_is_a_third_of_the_lease_with_a_one_millisecond_floor() {
    assert_eq!(Lease { duration: Duration::from_secs(30), watchdog: true }.renewal_interval(), Duration::from_secs(10));
    assert_eq!(Lease { duration: Duration::from_millis(2), watchdog: true }.renewal_interval(), Duration::from_millis(1));
    assert_eq!(Lease { duration: Duration::from_millis(2), watchdog: false }.renewal_interval(), Duration::from_millis(1));
}

#[test]
fn a_guard_with_more_than_a_quorum_of_lost_leases_is_lost() {
    // 3 nodes, quorum 2: one lost lease is tolerated, two are not.
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::ownership lock::lease lock::guard`
Expected: compile errors.

- [ ] **Step 3: Implement**

`LockOwner` holds `client_id: Arc<str>` (a `Uuid::new_v4()` string created once per service) and `sequence: Arc<AtomicU64>`; `next_token` formats `"{client_id}:{sequence}"`. The colon is the Redisson field separator, so a token is also a valid hash field on the Java side. Both traits get `#[async_trait]`-style boxing: `LockBackend` uses `#[async_trait]`, `BlockingLockBackend` does not.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::ownership lock::lease lock::guard`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/backend next-web-data/next-web-data-redis/src/lock/ownership.rs next-web-data/next-web-data-redis/src/lock/lease.rs next-web-data/next-web-data-redis/src/lock/guard.rs
git commit -m "feat(redis-lock): add the backend traits, owner tokens and lease bookkeeping"
```

---

### Task 5: `RedisTemplate` implements both backend traits

**Files:**
- Modify: `next-web-data/next-web-data-redis/src/core/redis_template.rs`
- Modify: `next-web-data/next-web-data-redis/Cargo.toml` (`tokio = { workspace = true, features = ["sync", "time"] }`)

**Interfaces:**
- Consumes: `LockBackend`, `BlockingLockBackend`, `LockRequest`, `ScriptReply`.
- Produces:
  - `#[cfg(feature = "distributed-lock")] #[async_trait] impl LockBackend for RedisTemplate`
  - `#[cfg(feature = "distributed-lock")] impl BlockingLockBackend for RedisTemplate`
  - A private `RedisTemplate::lock_connections: Arc<tokio::sync::OnceCell<MultiplexedConnection>>` field under the same cfg
  - A private `RedisTemplate::blocking_lock_connection: Arc<Mutex<Option<redis::Connection>>>` field under the same cfg

- [ ] **Step 1: Write the failing test**

Add to `redis_template.rs`:

```rust
#[cfg(all(test, feature = "distributed-lock"))]
mod lock_backend_tests {
    use super::*;
    use crate::lock::backend::{BlockingLockBackend, LockBackend};
    use crate::lock::protocol::commands::status;

    #[test]
    fn the_template_is_usable_as_both_kinds_of_backend() {
        fn assert_async<T: LockBackend>() {}
        fn assert_blocking<T: BlockingLockBackend>() {}
        assert_async::<RedisTemplate>();
        assert_blocking::<RedisTemplate>();
    }

    #[test]
    fn the_direct_backend_publishes_with_publish() {
        // with_url only parses the URL, so this test needs no live server.
        let template = RedisTemplate::with_url("redis://127.0.0.1:6379/0").unwrap();
        // Two traits in scope define the same method name, so qualify the call.
        assert_eq!(<RedisTemplate as LockBackend>::publish_command(&template), "PUBLISH");
        assert_eq!(<RedisTemplate as BlockingLockBackend>::publish_command(&template), "PUBLISH");
        assert_eq!(status("orders", "c:1").keys(), ["orders".to_string()].as_slice());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib redis_template`
Expected: compile error — the trait is not implemented for `RedisTemplate`.

- [ ] **Step 3: Implement**

- Async: `eval` clones the cached `MultiplexedConnection` (created once through `get_or_try_init` using the existing `connection_config`, so a lock command no longer opens a TCP connection) and wraps `request.invoke_async(&mut connection)` in `tokio::time::timeout(timeout, ...)`, mapping elapsed to `RedisLockError::CommandTimeout { script: request.script().name() }`.
- Async `wait_for_unlock`: `client.get_async_pubsub()`, `subscribe(channel)`, then a `status` script call on `name` (Task 3), then `tokio::time::timeout(timeout, messages.next())`, ignoring the timeout. The existence check must be issued **after** subscribing so an unlock between the check and the wait cannot be missed.
- Blocking: take the `Mutex<Option<Connection>>`, lazily create it with `client.get_connection_with_timeout(timeout)`, call `set_read_timeout(Some(timeout))`, run `request.invoke(&mut *connection)`. On a `RedisError` that is an IO error, drop the cached connection so the next call reconnects; return the error either way. `wait_for_unlock` uses `std::thread::sleep(timeout)` after the same existence check.
- `publish_command` keeps the default `"PUBLISH"` for a direct client.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib redis_template`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/core/redis_template.rs next-web-data/next-web-data-redis/Cargo.toml
git commit -m "feat(redis-lock): implement the lock backend traits on redis template"
```

---

### Task 6: Cluster backend

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/backend/cluster.rs`
- Modify: `next-web-data/next-web-data-redis/src/lock/backend/mod.rs` (declare the module)

**Interfaces:**
- Consumes: both backend traits.
- Produces: `#[derive(Clone)] pub struct ClusterLockBackend` with `pub fn new(urls: Vec<String>, command_timeout: Duration) -> Result<Self, RedisLockError>` and `impl LockBackend` + `impl BlockingLockBackend`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_url_list_is_rejected() {
        assert!(matches!(
            ClusterLockBackend::new(Vec::new(), Duration::from_secs(3)),
            Err(RedisLockError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn a_cluster_backend_publishes_with_spublish() {
        let backend = ClusterLockBackend::new(vec!["redis://127.0.0.1:7000".into()], Duration::from_secs(3)).unwrap();
        assert_eq!(backend.publish_command(), "SPUBLISH");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::backend::cluster`
Expected: compile error.

- [ ] **Step 3: Implement**

Wrap `redis::cluster::ClusterClient`; `eval` uses `get_async_connection()` / `get_connection()` plus `request.invoke_async` / `request.invoke` inside the command timeout. `wait_for_unlock` sleeps for the timeout instead of subscribing, because a RESP2 cluster subscription has to own a node-addressed connection; the lock key and channel share a hash tag, so polling remains correct — document that with a code comment. `publish_command` returns `"SPUBLISH"`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::backend::cluster`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/backend/cluster.rs next-web-data/next-web-data-redis/src/lock/backend/mod.rs
git commit -m "feat(redis-lock): add the cluster lock backend"
```

---

### Task 7: In-memory test double

Every behaviour test from Task 8 onward runs against this double, so no test in `cargo test` needs a live Redis. It must emulate the five scripts exactly, including the return codes, because it is also the executable documentation of the protocol.

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/backend/memory.rs` (`#[cfg(test)]`)
- Modify: `next-web-data/next-web-data-redis/src/lock/backend/mod.rs`

**Interfaces:**
- Consumes: both backend traits, `LockRequest`, `ScriptArg`, `ScriptReply`.
- Produces:
  - `#[cfg(test)] #[derive(Clone, Default)] pub struct MemoryLockBackend`
  - `pub fn new_shared() -> Arc<Self>`, `pub fn fail_next_eval(&self)`, `pub fn set_unavailable(&self, unavailable: bool)`, `pub fn expire_now(&self, name: &str)`, `pub fn hold_count(&self, name: &str, owner: &str) -> u32`, `pub fn renew_count(&self, name: &str) -> u32`, `pub fn exists(&self, name: &str) -> bool`, `pub fn unlock_notifications(&self) -> Vec<String>`, `pub fn set_eval_delay(&self, delay: Duration)`
  - `impl LockBackend` and `impl BlockingLockBackend`, both enforcing the `timeout` argument the way a real backend does (see Step 3)

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn the_double_emulates_the_acquire_and_unlock_contract() {
    let backend = MemoryLockBackend::new_shared();
    let acquire = acquire("orders", "c:1", Duration::from_secs(30)).unwrap();
    assert_eq!(backend.eval(&acquire, Duration::from_secs(1)).await.unwrap(), ScriptReply::Nil);
    let contended = acquire("orders", "c:2", Duration::from_secs(30)).unwrap();
    assert!(matches!(
        backend.eval(&contended, Duration::from_secs(1)).await.unwrap(),
        ScriptReply::Int(ttl) if ttl > 0
    ));

    let unlock = unlock("orders", "c:1", Duration::from_secs(30), "PUBLISH").unwrap();
    assert_eq!(backend.eval(&unlock, Duration::from_secs(1)).await.unwrap().int(), Some(1));
    assert_eq!(backend.unlock_notifications(), vec!["redisson_lock__channel:{orders}".to_string()]);
    assert!(!backend.exists("orders"));
}

#[tokio::test]
async fn the_double_rejects_a_foreign_unlock_without_touching_state() {
    let backend = MemoryLockBackend::new_shared();
    backend.eval(&acquire("orders", "c:1", Duration::from_secs(30)).unwrap(), Duration::from_secs(1)).await.unwrap();
    let foreign = unlock("orders", "c:2", Duration::from_secs(30), "PUBLISH").unwrap();
    assert_eq!(backend.eval(&foreign, Duration::from_secs(1)).await.unwrap().int(), Some(-1));
    assert_eq!(backend.hold_count("orders", "c:1"), 1);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::backend::memory`
Expected: compile error.

- [ ] **Step 3: Implement**

State is `Mutex<HashMap<String, MemoryLock>>` where `MemoryLock { counts: HashMap<String, u32>, expires_at: Option<Instant>, channel: String }`. `eval` matches on `request.script().name()`, honours an optional `eval_delay` and a one-shot `fail_next_eval` flag, and reproduces the five return contracts. It also enforces `timeout`: the async flavour races `eval_delay` against `tokio::time::timeout` and returns `CommandTimeout`; the blocking flavour sleeps then compares elapsed time and returns the same error. Without that, no test could prove that a hung node fails in bounded time. `wait_for_unlock` returns immediately when `name` no longer exists, otherwise waits on a `tokio::sync::Notify` registered per channel for the async flavour and polls a `Condvar`-backed counter for the blocking one; the notification list is recorded for assertions. Expiry is checked lazily on every access.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::backend::memory`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/backend/memory.rs next-web-data/next-web-data-redis/src/lock/backend/mod.rs
git commit -m "test(redis-lock): add the in-memory backend that emulates the lua scripts"
```

---

### Task 8: Async lock

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/async_lock/mod.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/async_lock/lock.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/async_lock/guard.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/async_lock/watchdog.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/async_lock/waiter.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/test_support.rs` (`#[cfg(test)]`)
- Modify: `next-web-data/next-web-data-redis/src/lock/mod.rs`

**Interfaces:**
- Consumes: `Arc<dyn LockBackend>`, `LockRequest` builders, `Lease`, `GuardCore`, `NodeLease`, `OwnerToken`.
- Produces:
  - `#[derive(Clone)] pub struct RedisLock` with `pub fn new(backend: Arc<dyn LockBackend>, name: impl Into<String>, owner: OwnerToken, config: Arc<RedisLockConfig>) -> Self` and the full method set from "API Surface".
  - `pub struct RedisLockGuard` with `pub fn name(&self) -> &str`, `pub fn owner_id(&self) -> &str`, `pub fn lease(&self) -> Duration`, `pub fn is_lost(&self) -> bool`, `pub async fn unlock(self) -> Result<UnlockOutcome, RedisLockError>`; `Drop` cancels the watchdog and logs a `warn!`.
  - `pub(crate) fn retry_delay(remaining: Duration, retry_interval: Duration, observed_ttl: Option<Duration>) -> Duration` — `max(1 ms, min(remaining, retry_interval, observed_ttl))`.

Acquire algorithm (the only place the retry policy lives):

1. `deadline = wait.map(|w| Instant::now() + w)`; reject a zero wait or zero lease with `InvalidDuration`.
2. Loop: build `acquire(name, owner, lease)`, `eval` it.
3. `ScriptReply::Nil` → acquired. Create `NodeLease` per acquired node, spawn one watchdog task per watchdog lease, return the guard.
4. `ScriptReply::Int(ttl)` → contended. If `wait` is `None`, return `Ok(None)`. If the deadline passed, return `Ok(None)`. Otherwise `wait_for_unlock(name, channel, retry_delay(remaining, config.retry_interval, Some(ttl)))` and loop.
5. Any `RedisLockError` other than a timeout aborts with `Err`.

- [ ] **Step 1: Write the failing tests**

All against `MemoryLockBackend`:

```rust
#[tokio::test]
async fn two_owners_are_mutually_exclusive() {
    let backend = MemoryLockBackend::new_shared();
    let first = test_lock(backend.clone(), "orders");
    let second = test_lock(backend.clone(), "orders");
    let guard = first.try_lock().await.unwrap().unwrap();
    assert!(second.try_lock().await.unwrap().is_none());
    assert!(second.is_locked().await.unwrap());
    guard.unlock().await.unwrap();
    assert!(second.try_lock().await.unwrap().is_some());
}

#[tokio::test]
async fn second_lock_on_the_same_handle_is_reentrant() {
    let backend = MemoryLockBackend::new_shared();
    let lock = test_lock(backend.clone(), "orders");
    let first = lock.try_lock().await.unwrap().unwrap();
    let second = lock.clone().try_lock().await.unwrap().unwrap();
    assert_eq!(lock.hold_count().await.unwrap(), 2);
    assert_eq!(&first.owner_id(), &second.owner_id());
    assert_eq!(second.unlock().await.unwrap(), UnlockOutcome::Reentrant);
    assert_eq!(first.unlock().await.unwrap(), UnlockOutcome::Released);
    assert_eq!(backend.unlock_notifications().len(), 1);
}

#[tokio::test]
async fn force_unlock_releases_a_lock_the_caller_does_not_own() {
    let backend = MemoryLockBackend::new_shared();
    let owner = test_lock(backend.clone(), "orders");
    let intruder = test_lock(backend.clone(), "orders");
    let guard = owner.try_lock().await.unwrap().unwrap();
    assert!(!intruder.is_held_by_owner().await.unwrap());
    assert!(intruder.force_unlock().await.unwrap());
    assert!(!owner.is_locked().await.unwrap());
    // The owner's hash field is gone, so releasing that level is refused.
    assert!(matches!(guard.unlock().await, Err(RedisLockError::NotOwner)));
}

#[tokio::test]
async fn try_lock_for_waits_and_succeeds_after_release() {
    let backend = MemoryLockBackend::new_shared();
    let holder = test_lock(backend.clone(), "orders");
    let waiter = test_lock(backend.clone(), "orders");
    let guard = holder.try_lock().await.unwrap().unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        guard.unlock().await.unwrap();
    });
    let acquired = waiter.try_lock_for(Duration::from_secs(2)).await.unwrap();
    assert!(acquired.is_some());
    release.await.unwrap();
}

#[tokio::test]
async fn try_lock_for_respects_the_wait_budget() {
    let backend = MemoryLockBackend::new_shared();
    let holder = test_lock(backend.clone(), "orders");
    let _guard = holder.try_lock().await.unwrap().unwrap();
    let started = Instant::now();
    assert!(test_lock(backend.clone(), "orders").try_lock_for(Duration::from_millis(150)).await.unwrap().is_none());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn watchdog_renews_until_unlock() {
    let backend = MemoryLockBackend::new_shared();
    let config = RedisLockConfig { watchdog_timeout: Duration::from_millis(90), ..Default::default() };
    let lock = RedisLock::new(backend.clone(), "orders", new_owner_token(), Arc::new(config));
    let guard = lock.lock().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!guard.is_lost());
    assert!(lock.is_held_by_owner().await.unwrap());
    assert!(backend.renew_count("orders") > 3);
    guard.unlock().await.unwrap();
}

#[tokio::test]
async fn lost_lease_is_reported_on_unlock() {
    let backend = MemoryLockBackend::new_shared();
    let config = RedisLockConfig { watchdog_timeout: Duration::from_millis(90), ..Default::default() };
    let lock = RedisLock::new(backend.clone(), "orders", new_owner_token(), Arc::new(config));
    let guard = lock.lock().await.unwrap();
    backend.expire_now("orders");
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(guard.is_lost());
    assert!(matches!(guard.unlock().await, Err(RedisLockError::WatchdogLost)));
}

#[tokio::test]
async fn eval_timeout_surfaces_as_error() {
    let backend = MemoryLockBackend::new_shared();
    backend.set_eval_delay(Duration::from_millis(300));
    let config = RedisLockConfig { command_timeout: Duration::from_millis(50), ..Default::default() };
    let lock = RedisLock::new(backend, "orders", new_owner_token(), Arc::new(config));
    assert!(matches!(lock.try_lock().await, Err(RedisLockError::CommandTimeout { .. })));
}

#[tokio::test]
async fn with_lock_releases_even_when_the_operation_fails() {
    let backend = MemoryLockBackend::new_shared();
    let lock = test_lock(backend.clone(), "orders");
    let result: Result<Result<(), &'static str>, _> = lock.with_lock(|| async { Err("boom") }).await;
    assert_eq!(result.unwrap(), Err("boom"));
    assert!(!lock.is_locked().await.unwrap());
}

#[test]
fn the_retry_delay_never_exceeds_its_budget() {
    // remaining caps the delay, the retry interval caps it further, and an
    // observed TTL caps it again; the floor is one millisecond so the loop spins.
    let remaining = Duration::from_secs(5);
    let retry = Duration::from_millis(100);
    assert_eq!(retry_delay(remaining, retry, None), retry);
    assert_eq!(retry_delay(remaining, retry, Some(Duration::from_millis(20))), Duration::from_millis(20));
    assert_eq!(retry_delay(Duration::from_millis(5), retry, None), Duration::from_millis(5));
    assert_eq!(retry_delay(Duration::ZERO, retry, None), Duration::from_millis(1));
}
```

Both `test_lock`, `test_blocking_lock` and `new_owner_token` are shared test helpers in `src/lock/test_support.rs`, declared as `#[cfg(test)] mod test_support;` from `lock/mod.rs` so Tasks 8, 9 and 10 all reach them. `new_owner_token()` is `LockOwner::default().next_token()` — there is deliberately no `Default` impl on `OwnerToken`, so production code cannot mint a token without a service. `test_lock(backend, name)` is `RedisLock::new(backend, name, new_owner_token(), Arc::new(RedisLockConfig::default()))`; Task 9 adds the blocking twin, which takes the same default config. Tests that need a specific watchdog or timeout call the constructor directly.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::async_lock`
Expected: compile error — types missing.

- [ ] **Step 3: Implement**

`lock.rs` holds the algorithm above. `guard.rs` owns the `watch::Sender<bool>` cancel handle, `Drop`, and the release path: cancel the watchdog first, then `eval(unlock(...))` and map `1`/`0`/`-1` to `Released`/`Reentrant`/`NotOwner`. When `is_lost()` is true the guard must **not** touch Redis at all — the lease is already gone and deleting the key could destroy the next owner's lock — it just returns `WatchdogLost`. That deliberately differs from the code being replaced, which re-ran a rollback on a lost lease. `watchdog.rs` spawns one task per acquired node that loops `select!` on the cancel channel and `sleep(renewal_interval)`, retries a renewal once, and on a second failure sets `NodeLease::lost` and logs `warn!`. `waiter.rs` is the pure `retry_delay` function plus its unit tests.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::async_lock`
Expected: 9 passed, plus the `retry_delay` unit tests in `waiter.rs`.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/async_lock next-web-data/next-web-data-redis/src/lock/mod.rs
git commit -m "feat(redis-lock): add the asynchronous reentrant lock"
```

---

### Task 9: Blocking lock

Same semantics as Task 8 through the synchronous client. No `block_on`: the state machine calls `BlockingLockBackend` directly and waits with `std::thread::sleep`. Wake-ups come from polling at `config.retry_interval` rather than from Pub/Sub, because a synchronous subscriber would need a second connection and a receive thread; the async API keeps the notification path. Document that difference on the type.

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/blocking/mod.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/blocking/lock.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/blocking/guard.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/blocking/watchdog.rs`
- Modify: `next-web-data/next-web-data-redis/src/lock/mod.rs`

**Interfaces:**
- Consumes: `Arc<dyn BlockingLockBackend>`, `GuardCore`, `NodeLease`.
- Produces: `#[derive(Clone)] pub struct BlockingRedisLock` with `pub fn new(backend: Arc<dyn BlockingLockBackend>, name: impl Into<String>, owner: OwnerToken, config: Arc<RedisLockConfig>) -> Self` and the blocking method set; `pub struct BlockingRedisLockGuard` with `pub fn name(&self) -> &str`, `pub fn owner_id(&self) -> &str`, `pub fn lease(&self) -> Duration`, `pub fn is_lost(&self) -> bool`, `pub fn unlock(self) -> Result<UnlockOutcome, RedisLockError>`.

- [ ] **Step 1: Write the failing tests**

Mirror Task 8's matrix with `#[test]` instead of `#[tokio::test]`, plus one test that the blocking lock never needs a runtime:

```rust
#[test]
fn blocking_two_owners_are_mutually_exclusive() {
    let backend = MemoryLockBackend::new_shared();
    let guard = test_blocking_lock(backend.clone(), "orders").try_lock().unwrap().unwrap();
    assert!(test_blocking_lock(backend.clone(), "orders").try_lock().unwrap().is_none());
    assert_eq!(guard.unlock().unwrap(), UnlockOutcome::Released);
}

#[test]
fn blocking_watchdog_renews_until_unlock() {
    let backend = MemoryLockBackend::new_shared();
    let config = RedisLockConfig { watchdog_timeout: Duration::from_millis(90), ..Default::default() };
    let lock = BlockingRedisLock::new(backend.clone(), "orders", new_owner_token(), Arc::new(config));
    let guard = lock.lock().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    assert!(!guard.is_lost());
    guard.unlock().unwrap();
}

#[test]
fn a_dropped_blocking_guard_stops_renewing() {
    let backend = MemoryLockBackend::new_shared();
    let lock = test_blocking_lock(backend.clone(), "orders");
    {
        let _guard = lock.lock().unwrap();
    }
    let renewals_after_drop = backend.renew_count("orders");
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(backend.renew_count("orders"), renewals_after_drop);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::blocking`
Expected: compile error.

- [ ] **Step 3: Implement**

`watchdog.rs` spawns `std::thread::spawn` with a `std::sync::mpsc::Receiver<()>`; the thread loops `rx.recv_timeout(renewal_interval)`, renews on timeout, and exits on `Ok` or `Disconnected`. `guard.unlock` and `Drop` both drop the sender. `lock.rs` mirrors Task 8's algorithm with `std::thread::sleep` on the retry path. `#[deny(clippy::await_holding_lock)]` is not needed (no async here), but the `Mutex<Option<Connection>>` in Task 5 must never be held across an await — it never is, because the blocking trait has no futures.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::blocking`
Expected: 10 passed (the async matrix mirrored plus the drop test; the double enforces the command timeout for both flavours, so the timeout test is included).

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/blocking next-web-data/next-web-data-redis/src/lock/mod.rs
git commit -m "feat(redis-lock): add the blocking reentrant lock"
```

---

### Task 10: RedLock

Quorum composition over at least three independent deployments. Async and blocking flavours share the quorum rule; each acquires nodes concurrently (async) or in sequence (blocking) and rolls back the partial acquisitions when the quorum or the validity check fails.

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/redlock/mod.rs`
- Create: `next-web-data/next-web-data-redis/src/lock/redlock/lock.rs`
- Modify: `next-web-data/next-web-data-redis/src/lock/mod.rs`

**Interfaces:**
- Consumes: `Vec<Arc<dyn LockBackend>>`, `Vec<Arc<dyn BlockingLockBackend>>`.
- Produces:
  - `pub struct RedLock` with `pub fn new(nodes: Vec<Arc<dyn LockBackend>>, name: impl Into<String>, owner: OwnerToken, config: Arc<RedisLockConfig>) -> Result<Self, RedisLockError>` plus the async method set; `pub fn quorum(&self) -> usize`.
  - `pub struct BlockingRedLock` with the same shape over `Vec<Arc<dyn BlockingLockBackend>>`.
  - `pub const fn quorum_for(node_count: usize) -> usize` — `node_count / 2 + 1`.
  - Guard types `RedLockGuard` and `BlockingRedLockGuard` holding `GuardCore` with one `NodeLease` per acquired node, each with `pub fn node_count(&self) -> usize`, `pub fn quorum(&self) -> usize`, `pub fn is_lost(&self) -> bool`, and a `pub async fn unlock(self)` / `pub fn unlock(self)` respectively returning `Result<UnlockOutcome, RedisLockError>`.

Algorithm: record `started = Instant::now()`; evaluate every node; the attempt succeeds when `acquired >= quorum` **and** `started.elapsed() + drift < lease`, where `drift = 2 ms + lease / 100`; on failure release every node acquired in that attempt before retrying, so a partial acquisition never lingers.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn quorum_is_a_strict_majority() {
    assert_eq!(quorum_for(3), 2);
    assert_eq!(quorum_for(4), 3);
    assert_eq!(quorum_for(5), 3);
}

#[test]
fn fewer_than_three_nodes_is_rejected() {
    // The binding's type drives the unsizing coercion of each element.
    let nodes: Vec<Arc<dyn LockBackend>> = vec![MemoryLockBackend::new_shared()];
    assert!(matches!(
        RedLock::new(nodes, "orders", new_owner_token(), Arc::new(RedisLockConfig::default())),
        Err(RedisLockError::InvalidConfiguration(_))
    ));
}

#[tokio::test]
async fn a_single_unavailable_node_still_reaches_the_quorum() {
    let healthy_a = MemoryLockBackend::new_shared();
    let healthy_b = MemoryLockBackend::new_shared();
    let broken = MemoryLockBackend::new_shared();
    broken.set_unavailable(true);
    let nodes: Vec<Arc<dyn LockBackend>> = vec![healthy_a, healthy_b, broken];
    let lock = RedLock::new(nodes, "orders", new_owner_token(), Arc::new(RedisLockConfig::default())).unwrap();
    let guard = lock.try_lock().await.unwrap().unwrap();
    assert_eq!(guard.node_count(), 2);
    guard.unlock().await.unwrap();
}

#[tokio::test]
async fn two_unavailable_nodes_fail_and_roll_the_third_back() {
    let healthy = MemoryLockBackend::new_shared();
    let broken_a = MemoryLockBackend::new_shared();
    let broken_b = MemoryLockBackend::new_shared();
    broken_a.set_unavailable(true);
    broken_b.set_unavailable(true);
    let nodes: Vec<Arc<dyn LockBackend>> = vec![healthy.clone(), broken_a, broken_b];
    let lock = RedLock::new(nodes, "orders", new_owner_token(), Arc::new(RedisLockConfig::default())).unwrap();
    assert!(lock.try_lock().await.unwrap().is_none());
    // The reachable node must not keep the lock after a failed attempt.
    assert!(!healthy.exists("orders"));
}

#[test]
fn a_blocking_red_lock_reaches_the_quorum() {
    let nodes: Vec<Arc<dyn BlockingLockBackend>> = vec![
        MemoryLockBackend::new_shared(),
        MemoryLockBackend::new_shared(),
        MemoryLockBackend::new_shared(),
    ];
    let lock = BlockingRedLock::new(nodes, "orders", new_owner_token(), Arc::new(RedisLockConfig::default())).unwrap();
    let guard = lock.try_lock().unwrap().unwrap();
    assert_eq!(guard.node_count(), 3);
    guard.unlock().unwrap();
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::redlock`
Expected: compile error.

- [ ] **Step 3: Implement**

`redlock/lock.rs` holds both flavours. The async attempt uses `futures::future::join_all`; the blocking one iterates. Rollback re-issues `force_unlock` on each node acquired during the failed attempt, ignoring individual errors but counting them for the `PartiallyReleased` error when the *successful* release path partially fails.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::redlock`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/redlock next-web-data/next-web-data-redis/src/lock/mod.rs
git commit -m "feat(redis-lock): add the quorum red lock in both flavours"
```

---

### Task 11: Service facade

**Files:**
- Create: `next-web-data/next-web-data-redis/src/lock/service.rs`
- Modify: `next-web-data/next-web-data-redis/src/lock/mod.rs`

**Interfaces:**
- Consumes: everything above.
- Produces:
  - `#[derive(Clone)] pub struct RedisLockService`
  - `pub fn new<B>(nodes: Vec<Arc<B>>, config: RedisLockConfig) -> Result<Self, RedisLockError> where B: LockBackend + BlockingLockBackend + 'static`
  - `pub fn from_template(template: &RedisTemplate, config: RedisLockConfig) -> Result<Self, RedisLockError>`
  - `pub fn from_cluster(urls: Vec<String>, config: RedisLockConfig) -> Result<Self, RedisLockError>`
  - `pub fn get_lock(&self, name: impl Into<String>) -> RedisLock`
  - `pub fn get_blocking_lock(&self, name: impl Into<String>) -> BlockingRedisLock`
  - `pub fn get_red_lock(&self, name: impl Into<String>) -> Result<RedLock, RedisLockError>`
  - `pub fn get_blocking_red_lock(&self, name: impl Into<String>) -> Result<BlockingRedLock, RedisLockError>`
  - `pub fn config(&self) -> &RedisLockConfig`, `pub fn node_count(&self) -> usize`, `pub fn client_id(&self) -> &str`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn one_service_mints_a_fresh_owner_token_per_handle() {
    let service = RedisLockService::new(vec![MemoryLockBackend::new_shared()], RedisLockConfig::default()).unwrap();
    let first = service.get_lock("orders");
    let second = service.get_lock("orders");
    // Two handles on one name contend by design: each mints its own token.
    assert_ne!(first.owner_id(), second.owner_id());
    // A clone of one handle keeps the token, which is what makes it reentrant.
    assert_eq!(first.owner_id(), first.clone().owner_id());
}

#[test]
fn a_single_node_service_offers_async_and_blocking_handles() {
    let service = RedisLockService::new(vec![MemoryLockBackend::new_shared()], RedisLockConfig::default()).unwrap();
    assert_eq!(service.node_count(), 1);
    let blocking = service.get_blocking_lock("orders");
    let guard = blocking.try_lock().unwrap().unwrap();
    assert_eq!(guard.owner_id(), blocking.owner_id());
    assert!(matches!(
        service.get_red_lock("orders"),
        Err(RedisLockError::InvalidConfiguration(_))
    ));
}

#[test]
fn an_invalid_configuration_is_rejected_at_construction() {
    let config = RedisLockConfig { command_timeout: Duration::ZERO, ..Default::default() };
    assert!(matches!(
        RedisLockService::new(vec![MemoryLockBackend::new_shared()], config),
        Err(RedisLockError::InvalidDuration { field: "command_timeout" })
    ));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::service`
Expected: compile error.

- [ ] **Step 3: Implement**

The service stores `nodes: Arc<[Arc<dyn LockBackend>]>`, `blocking_nodes: Arc<[Arc<dyn BlockingLockBackend>]>`, `config: Arc<RedisLockConfig>`, and `owner: LockOwner`. `new` validates the config and clones each `Arc<B>` into both trait objects. `get_lock` targets `nodes[0]`; `get_blocking_lock` targets `blocking_nodes[0]`; the red-lock constructors require `nodes.len() >= 3`. `from_template` wraps `Arc::new(template.clone())`; `from_cluster` builds one `ClusterLockBackend`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib lock::service`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/src/lock/service.rs next-web-data/next-web-data-redis/src/lock/mod.rs
git commit -m "feat(redis-lock): add the lock service facade"
```

---

### Task 12: Cutover

Delete the previous implementation and point the crate, the properties and the auto-configuration at the new one. Nothing here is new behaviour; the whole crate must be green at the end of the task.

**Files:**
- Delete: `next-web-data/next-web-data-redis/src/lock/redisson_lock.rs`, `redisson_base_lock.rs`, `redisson_expirable.rs`, `redisson_multi_lock.rs`, `redisson_object.rs`, `redisson_red_lock.rs`, `rlock.rs`, `redlock.rs`, `lock_pub_sub.rs`, `lock_renewal_scheduler.rs`, `lock_scripts.rs`, `scripts/` (the five old `.lua` copies)
- Rewrite: `next-web-data/next-web-data-redis/src/lock/mod.rs`
- Modify: `next-web-data/next-web-data-redis/src/lib.rs`
- Modify: `next-web-data/next-web-data-redis/src/autoconfigure/redis_properties.rs`
- Modify: `next-web-data/next-web-data-redis/src/autoconfigure/redis_auto_configuration.rs`
- Rewrite: `next-web-data/next-web-data-redis/tests/distributed_lock.rs` (API names only; the real-Redis coverage lands in Task 13)

**Interfaces:**
- Consumes: the new public surface.
- Produces: the crate root re-export block quoted in "API Surface", and `RedisLockProperties` with public fields plus `pub enum RedisLockMode`.

- [ ] **Step 1: Write the failing test**

In `tests/distributed_lock.rs`, the two behaviour tests become:

```rust
#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn direct_lock_is_reentrant_and_owner_safe() {
    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-lock:{}", uuid::Uuid::new_v4());
    let lock = service.get_lock(&name);
    let other_owner = service.get_lock(&name);

    let first = lock.try_lock().await.unwrap().unwrap();
    let second = lock.clone().try_lock().await.unwrap().unwrap();
    assert_eq!(lock.hold_count().await.unwrap(), 2);
    assert!(other_owner.try_lock().await.unwrap().is_none());
    assert_eq!(second.unlock().await.unwrap(), UnlockOutcome::Reentrant);
    assert_eq!(first.unlock().await.unwrap(), UnlockOutcome::Released);

    let acquired = other_owner.try_lock().await.unwrap().unwrap();
    acquired.unlock().await.unwrap();
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn a_blocking_lock_excludes_the_async_lock() {
    let service = test_service().expect("NEXT_WEB_TEST_REDIS_URL must contain a Redis URL");
    let name = format!("next-web:test-blocking:{}", uuid::Uuid::new_v4());
    let blocking = service.get_blocking_lock(&name);
    let guard = tokio::task::spawn_blocking(move || blocking.try_lock()).await.unwrap().unwrap().unwrap();
    assert!(service.get_lock(&name).try_lock().await.unwrap().is_none());
    tokio::task::spawn_blocking(move || guard.unlock()).await.unwrap().unwrap();
}
```

`test_service` becomes `RedisLockService::from_template(&RedisTemplate::with_url(url).ok()?, RedisLockConfig::default()).ok()`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p next-web-data-redis --features distributed-lock --test distributed_lock --no-run`
Expected: compile error — `UnlockOutcome`/`try_lock`/`from_template` do not exist on the old API.

- [ ] **Step 3: Delete the old implementation and rewire**

- Remove the files listed above.
- `lock/mod.rs` keeps only the new module declarations plus the re-export block from "API Surface" (including `ScriptReply`, `LockRequest`, `LockScript` and `ScriptArg`, because the backend traits name them), and gains a module doc describing the async/blocking split and the per-owner reentrancy rule.
- `lib.rs` replaces the `crate::lock::{LockError, LockGuard, RedisDistributedLockService, RedisLock, ...}` block with `pub use lock::{...}` matching the "API Surface" block exactly.
- `redis_properties.rs`: `RedisLockProperties` fields become `pub`, `RedisLockPropertyMode` becomes `pub` and loses its `lock_config` import of `RedisLockMode`; `lock_config()` returns only the three timing knobs.
- `redis_auto_configuration.rs`: replace the commented-out `redis_lock_auto_configuration` body with a real one — when `redis_properties.lock().enabled`, resolve `RedisTemplate` from the context, build the service from the mode (`Direct` → `from_template`, `Cluster` → `from_cluster(cluster.urls)`), and `ctx.insert_singleton_with_default_name(service)`. Keep the method fallible and propagate a configuration error as `Box<dyn Error>`.
- Leave `tests/java/RedissonInteropProbe.java` untouched: the wire format is unchanged.

- [ ] **Step 4: Verify the crate**

Run: `cargo test -p next-web-data-redis --features distributed-lock --lib`
Expected: every unit test passes and no test needs Redis.

Run: `cargo check -p next-web-data-redis --features distributed-lock --all-targets`
Expected: `Finished` with no warnings.

Run: `cargo check -p next-web-data-redis --no-default-features`
Expected: `Finished`.

- [ ] **Step 5: Commit**

```bash
git add -A next-web-data/next-web-data-redis
git commit -m "refactor(redis-lock): replace the single-file lock with the modular implementation"
```

---

### Task 13: Live-Redis coverage and documentation

**Files:**
- Modify: `next-web-data/next-web-data-redis/tests/distributed_lock.rs`
- Modify: `next-web-data/next-web-data-redis/README.md`

**Interfaces:**
- Consumes: the full public API.
- Produces: integration tests for every Lua script against a real server, and a README that documents both APIs.

- [ ] **Step 1: Add the integration tests**

Keep the existing env-driven tests and add:

```rust
#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn every_script_matches_the_in_memory_double() {
    // acquire → contends → renew keeps the lease → foreign unlock returns NotOwner →
    // owner unlock releases and notifies a subscriber on the channel.
}

#[tokio::test]
#[ignore = "requires NEXT_WEB_TEST_REDIS_URL"]
async fn an_explicit_lease_expires_without_a_watchdog() {
    // try_lock_for_with_lease(50ms, 150ms), sleep 250ms, then a competitor acquires.
    // The first guard's unlock must fail with Err(RedisLockError::NotOwner), not
    // WatchdogLost: a fixed lease has no watchdog to lose ownership, the field is
    // simply gone by the time the guard tries to release it.
}
```

- [ ] **Step 2: Run them against a real server**

Run: `$env:NEXT_WEB_TEST_REDIS_URL='redis://127.0.0.1:6379/0'; cargo test -p next-web-data-redis --features distributed-lock --test distributed_lock -- --ignored`
Expected: if no server is reachable, record that in the final report instead of claiming the tests passed — this machine has no Redis on 6379 and no Docker.

- [ ] **Step 3: Rewrite the README section**

Document: the two APIs side by side, the per-owner reentrancy rule, the watchdog versus explicit lease, why `LockGuard` must be unlocked explicitly (`Drop` cannot perform IO), the blocking API's threading rule (call it from a blocking context, never from an async task), cluster mode, the RedLock quorum, the env variables for the ignored tests, and the Redisson 4.7 wire compatibility statement that already exists.

- [ ] **Step 4: Final sweep**

Run: `cargo fmt -p next-web-data-redis`
Run: `cargo clippy -p next-web-data-redis --features distributed-lock --all-targets -- -D warnings`
Run: `cargo test -p next-web-data-redis --features distributed-lock --lib`
Expected: all three clean.

- [ ] **Step 5: Commit**

```bash
git add next-web-data/next-web-data-redis/tests/distributed_lock.rs next-web-data/next-web-data-redis/README.md
git commit -m "docs(redis-lock): cover the live redis behaviour and document both APIs"
```

---

## Self-Review

**Spec coverage.** Async API → Task 8. Blocking API → Task 9. Trait implemented by `redis_template.rs` → Tasks 4, 5. Lua-only atomicity → Task 3 and the "Wire Protocol" section, enforced by the `commands` tests. Module dispersion → File Structure, one responsibility per file. Redlock → Task 10. Configuration and YAML wiring → Tasks 2, 11, 12. Documentation → Task 13.

**Deliberate scope limits, stated so they are not mistaken for gaps.** `RReadWriteLock`, `RSemaphore`, `RCountDownLatch`, fair locks, conditions and `forceUnlock` on behalf of another thread are out of scope; the user asked for a working distributed lock, not API parity with Redisson. The blocking API polls instead of subscribing. RedLock needs three independent deployments, so it is rejected for a single-node service.

**Known risk carried into execution.** The in-memory double is the test oracle for Tasks 8–10, and it can drift from the Lua scripts it emulates. Task 13's `every_script_matches_the_in_memory_double` is the only guard against that drift, and it cannot run on this machine (no Redis, no Docker). If that test cannot be run before merge, say so explicitly in the final report rather than describing the lock as verified.

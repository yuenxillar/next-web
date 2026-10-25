# next-web-data-redis

Redis integration for next-web. Distributed locking is optional and is enabled
with the `distributed-lock` Cargo feature (it is in the default feature set).

## Distributed lock

```toml
next-web-data-redis = { path = "../next-web-data-redis", features = ["distributed-lock"] }
```

```yaml
next:
  data:
    redis:
      url: redis://127.0.0.1:6379/0
      lock:
        enabled: true
        mode: direct
        watchdog-timeout: 30s
        retry-interval: 100ms
        command-timeout: 3s
        cluster:
          urls: []
        redlock:
          urls: []
```

Resolve `RedisLockService` from the application context, or build one yourself:

```rust,ignore
use std::time::Duration;
use next_web_data_redis::{RedisLockConfig, RedisLockService};

let service = RedisLockService::from_template(&redis_template, RedisLockConfig::default())?;
let lock = service.get_lock("order:42");
let guard = lock.lock().await?;
// Protected operation.
guard.unlock().await?;
```

### Two APIs, one protocol

| | asynchronous | blocking |
|---|---|---|
| type | `RedisLock` | `BlockingRedisLock` |
| guard | `RedisLockGuard` (`unlock().await`) | `BlockingRedisLockGuard` (`unlock()`) |
| waiter wake-up | Pub/Sub notification | polling at `retry-interval` |
| needs a runtime | yes | no |

The blocking API calls the `redis` crate's **synchronous** client directly. It is
not a `block_on` wrapper over the async path, so it never needs a runtime — but
it does block the thread it runs on. Call it from a blocking context
(`tokio::task::spawn_blocking`, a worker thread, a synchronous `main`), never from
inside an async task.

Both flavours expose the same method set:

| method | waits? | lease |
|---|---|---|
| `lock()` | forever | watchdog |
| `lock_with_lease(lease)` | forever | fixed, no watchdog |
| `try_lock()` | no | watchdog |
| `try_lock_for(wait)` | up to `wait` | watchdog |
| `try_lock_for_with_lease(wait, lease)` | up to `wait` | fixed, no watchdog |

Plus `is_locked()`, `is_held_by_owner()`, `hold_count()`, `remaining_lease()`,
`force_unlock()`, and `with_lock(operation)`. `with_lock` always releases, and
the operation's own error wins over a release error.

### Reentrancy follows the handle

Redisson identifies a lock holder by thread. A lock here is routinely held across
`.await` points, where no thread identity survives, so a handle carries an owner
token instead:

```rust,ignore
let lock = service.get_lock("order:42");
let first = lock.try_lock().await?.unwrap();
let second = lock.clone().try_lock().await?.unwrap(); // reentrant: same token
assert_eq!(lock.hold_count().await?, 2);

let other = service.get_lock("order:42"); // new token: contends
assert!(other.try_lock().await?.is_none());
```

Do not mix lease kinds on one handle. Every acquisition level re-arms the key's
TTL, so the last one wins: taking `lock()` (30 s watchdog) and then
`try_lock_for_with_lease(w, 150ms)` and releasing the short one leaves the key
with a 150 ms TTL while the watchdog guard still believes it holds the lock for
another 10 s. Redisson's protocol does the same, so this is wire-compatible
rather than a bug, but it is the one way the per-handle rule can shorten a lease
you thought you had.

### Guards must be unlocked explicitly

`Drop` cannot perform a Redis round trip, so dropping a guard without unlocking
only stops the watchdog and logs a warning; the lease then expires on its own. A
guard whose lease was lost reports `RedisLockError::WatchdogLost` from `unlock()`
and sends nothing to Redis — the key may already belong to the next owner.

### Cluster

```yaml
lock:
  enabled: true
  mode: cluster
  cluster:
    urls:
      - redis://127.0.0.1:7000
      - redis://127.0.0.1:7001
```

Lock keys and their Pub/Sub channels share a hash tag, so both land in the same
slot. Cluster notification waiting degrades to polling: a RESP2 cluster
subscription needs a node-addressed connection of its own, and a release is never
made correct by the wake-up.

### Quorum lock (Redlock)

Configure at least three unique `redlock.urls` and the auto-configuration builds
the service over those deployments, so `get_red_lock` / `get_blocking_red_lock`
become available:

```rust,ignore
let lock = service.get_red_lock("order:42")?;
let guard = lock.try_lock_for(Duration::from_secs(2)).await?;
```

A quorum is a strict majority of the nodes. An attempt that reaches a majority
*and* whose elapsed time plus drift still fits inside the lease succeeds;
otherwise every node it acquired is released before the next attempt, so a
partial acquisition never lingers. Fewer than three nodes is rejected: a majority
of two is not a quorum worth trusting.

### Compatibility

The lock hash, owner field, channel names, Lua behaviour, unlock message and
watchdog timing follow Redisson 4.7.0 core `RLock`, so a Java Redisson client and
this crate exclude each other; `tests/java/RedissonInteropProbe.java` proves it.
Spring integration, fair locks, read/write locks, conditions and semaphores are
not included.

### Tests

Unit tests run against an in-memory backend that emulates the five Lua scripts,
so `cargo test` needs no Redis:

```text
cargo test -p next-web-data-redis --features distributed-lock --lib
```

The live-Redis tests are ignored by default and read `NEXT_WEB_TEST_REDIS_URL`,
comma-separated `NEXT_WEB_TEST_REDIS_CLUSTER_URLS`, and comma-separated
`NEXT_WEB_TEST_REDLOCK_URLS`. The Java interoperability test additionally reads
`NEXT_WEB_TEST_REDISSON_CLASSPATH`, which must contain the local Redisson 4.7.0
core build and its runtime dependencies.

```text
cargo test -p next-web-data-redis --features distributed-lock -- --ignored
```

`every_script_matches_the_in_memory_double` is the one test that keeps the
in-memory emulation honest against the real scripts. It is also the only test
that can catch a divergence between the two, and it requires a live server.

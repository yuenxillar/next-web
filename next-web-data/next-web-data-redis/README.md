# next-web-data-redis

Redis integration for next-web. Distributed locking is optional and is enabled
with the `distributed-lock` Cargo feature.

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

Resolve `RedisDistributedLockService` from the application context, then create
a lock handle. A handle and its clones share an owner ID and are reentrant.
Calling `get_lock` again creates a different owner.

```rust,ignore
use next_web_data_redis::RedisDistributedLockService;

async fn update_order(locks: &RedisDistributedLockService) -> Result<(), Box<dyn std::error::Error>> {
    let lock = locks.get_lock("order:42");
    let guard = lock.acquire().await?;

    // Protected operation.

    guard.unlock().await?;
    Ok(())
}
```

`acquire` and `try_acquire_for` use a watchdog and renew the default 30-second
lease every 10 seconds. The `*_with_lease` methods use a fixed lease and do not
start a watchdog. `LockGuard` cannot perform async Redis I/O from `Drop`, so it
must be released with `unlock().await`; otherwise renewal stops and TTL expiry
is the final recovery mechanism. `with_lock` is available when a scoped closure
fits the operation.

Cluster mode uses the Redis crate's async Cluster client:

```yaml
lock:
  enabled: true
  mode: cluster
  cluster:
    urls:
      - redis://127.0.0.1:7000
      - redis://127.0.0.1:7001
```

Configure at least three unique `redlock.urls` to enable `get_redlock`. RedLock
uses a majority quorum. Redisson 4.7 marks `RedissonRedLock` as deprecated, so
standard `get_lock` remains the default and RedLock must be selected explicitly.

The lock hash, owner field, channel names, Lua behavior, unlock message, and
watchdog timing follow Redisson 4.7.0 core `RLock` behavior. Spring, MyBatis,
fair locks, read/write locks, conditions, and semaphores are not included.

External Redis tests read `NEXT_WEB_TEST_REDIS_URL`, comma-separated
`NEXT_WEB_TEST_REDIS_CLUSTER_URLS`, and comma-separated
`NEXT_WEB_TEST_REDLOCK_URLS`; they are ignored by default.
The Java interoperability test additionally reads
`NEXT_WEB_TEST_REDISSON_CLASSPATH`; it must contain the local Redisson 4.7.0
core build and its runtime dependencies.

```text
cargo test -p next-web-data-redis --features distributed-lock -- --ignored
```

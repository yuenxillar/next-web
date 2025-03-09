use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use deadpool_redis::redis::Value::Okay;
use deadpool_redis::redis::{Client, IntoConnectionInfo, RedisResult, Value};
use futures::future::join_all;
use futures::Future;
use rand::{thread_rng, Rng, RngCore};

const DEFAULT_RETRY_COUNT: u32 = 3;
const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(200);
const CLOCK_DRIFT_FACTOR: f32 = 0.01;
const UNLOCK_SCRIPT: &str = r#"
if redis.call("GET", KEYS[1]) == ARGV[1] then
  return redis.call("DEL", KEYS[1])
else
  return 0
end
"#;
const EXTEND_SCRIPT: &str = r#"
if redis.call("get", KEYS[1]) ~= ARGV[1] then
  return 0
else
  if redis.call("set", KEYS[1], ARGV[1], "PX", ARGV[2]) ~= nil then
    return 1
  else
    return 0
  end
end
"#;

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] deadpool_redis::redis::RedisError),

    #[error("Resource is unavailable")]
    Unavailable,

    #[error("TTL exceeded")]
    TtlExceeded,

    #[error("TTL too large")]
    TtlTooLarge,

    #[error("Redis connection failed for all servers")]
    RedisConnectionFailed,

    #[error("Redis key mismatch: expected value does not match actual value")]
    RedisKeyMismatch,

    #[error("Redis key not found")]
    RedisKeyNotFound,
}

/// The lock manager.
///
/// Implements the necessary functionality to acquire and release locks
/// and handles the Redis connections.
#[derive(Debug, Clone)]
pub struct RedisLockManager {
    lock_manager_inner: Arc<LockManagerInner>,
    retry_count: u32,
    retry_delay: Duration,
}

#[derive(Debug, Clone)]
struct LockManagerInner {
    /// List of all Redis clients
    pub servers: Vec<Client>,
    quorum: u32,
}

/// A distributed lock that can be acquired and released across multiple Redis instances.
///
/// A `Lock` represents a distributed lock in Redis.
/// The lock is associated with a resource, identified by a unique key, and a value that identifies
/// the lock owner. The `LockManager` is responsible for managing the acquisition, release, and extension
/// of locks.
#[derive(Debug)]
pub struct Lock {
    /// The resource to lock. Will be used as the key in Redis.
    pub resource: Vec<u8>,
    /// The value for this lock.
    pub val: Vec<u8>,
    /// Time the lock is still valid.
    /// Should only be slightly smaller than the requested TTL.
    pub validity_time: usize,
    /// Used to limit the lifetime of a lock to its lock manager.
    pub lock_manager: RedisLockManager,
}

/// Upon dropping the guard, `LockManager::unlock` will be ran synchronously on the executor.
///
/// This is known to block the tokio runtime if this happens inside of the context of a tokio runtime
/// if `tokio-comp` is enabled as a feature on this crate or the `redis` crate.
///
/// To eliminate this risk, if the `tokio-comp` flag is enabled, the `Drop` impl will not be compiled,
/// meaning that dropping the `LockGuard` will be a no-op.
/// Under this circumstance, `LockManager::unlock` can be called manually using the inner `lock` at the appropriate
/// point to release the lock taken in `Redis`.
#[derive(Debug)]
pub struct LockGuard {
    pub lock: Lock,
}

/// Dropping this guard inside the context of a tokio runtime if `tokio-comp` is enabled
/// will block the tokio runtime.
/// Because of this, the guard is not compiled if `tokio-comp` is enabled.
// #[cfg(not(feature = "tokio-comp"))]
impl Drop for LockGuard {
    fn drop(&mut self) {
        futures::executor::block_on(self.lock.lock_manager.unlock(&self.lock));
    }
}

impl RedisLockManager {
    /// Create a new lock manager instance, defined by the given Redis connection uris.
    ///
    /// Sample URI: `"redis://127.0.0.1:6379"`
    pub fn new<T: IntoConnectionInfo>(uris: Vec<T>) -> RedisLockManager {
        let servers: Vec<Client> = uris
            .into_iter()
            .map(|uri| Client::open(uri).unwrap())
            .collect();

        Self::from_clients(servers)
    }

    /// Create a new lock manager instance, defined by the given Redis clients.
    /// Quorum is defined to be N/2+1, with N being the number of given Redis instances.
    pub fn from_clients(clients: Vec<Client>) -> RedisLockManager {
        let quorum = (clients.len() as u32) / 2 + 1;

        RedisLockManager {
            lock_manager_inner: Arc::new(LockManagerInner {
                servers: clients,
                quorum,
            }),
            retry_count: DEFAULT_RETRY_COUNT,
            retry_delay: DEFAULT_RETRY_DELAY,
        }
    }

    /// Get 20 random bytes from the pseudorandom interface.
    pub fn get_unique_lock_id(&self) -> io::Result<Vec<u8>> {
        let mut buf = [0u8; 20];
        thread_rng().fill_bytes(&mut buf);
        Ok(buf.to_vec())
    }

    /// Set retry count and retry delay.
    ///
    /// Retry count defaults to `3`.
    /// Retry delay defaults to `200`.
    pub fn set_retry(&mut self, count: u32, delay: Duration) {
        self.retry_count = count;
        self.retry_delay = delay;
    }

    async fn lock_instance(
        client: &deadpool_redis::redis::Client,
        resource: &[u8],
        val: Vec<u8>,
        ttl: usize,
    ) -> bool {
        let mut con = match client.get_multiplexed_async_connection().await {
            Err(_) => return false,
            Ok(val) => val,
        };
        let result: RedisResult<Value> = deadpool_redis::redis::cmd("SET")
            .arg(resource)
            .arg(val)
            .arg("NX")
            .arg("PX")
            .arg(ttl)
            .query_async(&mut con)
            .await;

        match result {
            Ok(Okay) => true,
            Ok(_) | Err(_) => false,
        }
    }

    async fn extend_lock_instance(
        client: &deadpool_redis::redis::Client,
        resource: &[u8],
        val: &[u8],
        ttl: usize,
    ) -> bool {
        let mut con = match client.get_multiplexed_async_connection().await {
            Err(_) => return false,
            Ok(val) => val,
        };
        let script = deadpool_redis::redis::Script::new(EXTEND_SCRIPT);
        let result: RedisResult<i32> = script
            .key(resource)
            .arg(val)
            .arg(ttl)
            .invoke_async(&mut con)
            .await;
        match result {
            Ok(val) => val == 1,
            Err(_) => false,
        }
    }

    async fn unlock_instance(
        client: &deadpool_redis::redis::Client,
        resource: &[u8],
        val: &[u8],
    ) -> bool {
        let mut con = match client.get_multiplexed_async_connection().await {
            Err(_) => return false,
            Ok(val) => val,
        };
        let script = deadpool_redis::redis::Script::new(UNLOCK_SCRIPT);
        let result: RedisResult<i32> = script.key(resource).arg(val).invoke_async(&mut con).await;
        match result {
            Ok(val) => val == 1,
            Err(_) => false,
        }
    }

    // Can be used for creating or extending a lock
    async fn exec_or_retry<'a, T, Fut>(
        &'a self,
        resource: &[u8],
        value: &[u8],
        ttl: usize,
        lock: T,
    ) -> Result<Lock, LockError>
    where
        T: Fn(&'a Client) -> Fut,
        Fut: Future<Output = bool>,
    {
        for _ in 0..self.retry_count {
            let start_time = Instant::now();
            let n = join_all(self.lock_manager_inner.servers.iter().map(&lock))
                .await
                .into_iter()
                .fold(0, |count, locked| if locked { count + 1 } else { count });

            let drift = (ttl as f32 * CLOCK_DRIFT_FACTOR) as usize + 2;
            let elapsed = start_time.elapsed();
            let elapsed_ms =
                elapsed.as_secs() as usize * 1000 + elapsed.subsec_nanos() as usize / 1_000_000;
            if ttl <= drift + elapsed_ms {
                return Err(LockError::TtlExceeded);
            }
            let validity_time = ttl
                - drift
                - elapsed.as_secs() as usize * 1000
                - elapsed.subsec_nanos() as usize / 1_000_000;

            if n >= self.lock_manager_inner.quorum && validity_time > 0 {
                return Ok(Lock {
                    lock_manager: self.clone(),
                    resource: resource.to_vec(),
                    val: value.to_vec(),
                    validity_time,
                });
            } else {
                join_all(
                    self.lock_manager_inner
                        .servers
                        .iter()
                        .map(|client| Self::unlock_instance(client, resource, value)),
                )
                .await;
            }

            let retry_delay: u64 = self
                .retry_delay
                .as_millis()
                .try_into()
                .map_err(|_| LockError::TtlTooLarge)?;
            let n = thread_rng().gen_range(0..retry_delay);
            tokio::time::sleep(Duration::from_millis(n)).await
        }

        Err(LockError::Unavailable)
    }

    // Query Redis for a key's value and keep trying each server until a successful result is returned
    async fn query_redis_for_key_value(
        &self,
        resource: &[u8],
    ) -> Result<Option<Vec<u8>>, LockError> {
        for client in &self.lock_manager_inner.servers {
            let mut con = match client.get_multiplexed_async_connection().await {
                Ok(con) => con,
                Err(_) => continue, // If connection fails, try the next server
            };

            let result: RedisResult<Option<Vec<u8>>> = deadpool_redis::redis::cmd("GET")
                .arg(resource)
                .query_async(&mut con)
                .await;

            match result {
                Ok(val) => return Ok(val),
                Err(_) => continue, // If query fails, try the next server
            }
        }

        Err(LockError::RedisConnectionFailed) // All servers failed
    }

    /// Unlock the given lock.
    ///
    /// Unlock is best effort. It will simply try to contact all instances
    /// and remove the key.
    pub async fn unlock(&self, lock: &Lock) {
        join_all(
            self.lock_manager_inner
                .servers
                .iter()
                .map(|client| Self::unlock_instance(client, &lock.resource, &lock.val)),
        )
        .await;
    }

    /// Acquire the lock for the given resource and the requested TTL.
    ///
    /// If it succeeds, a `Lock` instance is returned,
    /// including the value and the validity time
    ///
    /// If it fails. `None` is returned.
    /// A user should retry after a short wait time.
    ///
    /// May return `LockError::TtlTooLarge` if `ttl` is too large.
    pub async fn lock<T: AsRef<[u8]>>(
        &self,
        resource: T,
        ttl: Duration,
    ) -> Result<Lock, LockError> {
        let val = self.get_unique_lock_id().map_err(LockError::Io)?;
        let ttl = ttl
            .as_millis()
            .try_into()
            .map_err(|_| LockError::TtlTooLarge)?;
        let resource = resource.as_ref();
        self.exec_or_retry(resource, &val.clone(), ttl, move |client| {
            Self::lock_instance(client, resource, val.clone(), ttl)
        })
        .await
    }

    /// Loops until the lock is acquired.
    ///
    /// The lock is placed in a guard that will unlock the lock when the guard is dropped.
    ///
    /// May return `LockError::TtlTooLarge` if `ttl` is too large.
    pub async fn acquire(&self, resource: &[u8], ttl: Duration) -> Result<LockGuard, LockError> {
        let lock = self.acquire_no_guard(resource, ttl).await?;
        Ok(LockGuard { lock })
    }

    /// Loops until the lock is acquired.
    ///
    /// Either lock's value must expire after the ttl has elapsed,
    /// or `LockManager::unlock` must be called to allow other clients to lock the same resource.
    ///
    /// May return `LockError::TtlTooLarge` if `ttl` is too large.
    pub async fn acquire_no_guard<T: AsRef<[u8]>>(
        &self,
        resource: T,
        ttl: Duration,
    ) -> Result<Lock, LockError> {
        loop {
            match self.lock(resource.as_ref(), ttl).await {
                Ok(lock) => return Ok(lock),
                Err(LockError::TtlTooLarge) => return Err(LockError::TtlTooLarge),
                Err(_) => continue,
            }
        }
    }

    /// Extend the given lock by given time in milliseconds
    pub async fn extend(&self, lock: &Lock, ttl: Duration) -> Result<Lock, LockError> {
        let ttl = ttl
            .as_millis()
            .try_into()
            .map_err(|_| LockError::TtlTooLarge)?;

        self.exec_or_retry(&lock.resource, &lock.val, ttl, move |client| {
            Self::extend_lock_instance(client, &lock.resource, &lock.val, ttl)
        })
        .await
    }

    /// Checks if the given lock has been freed (i.e., is no longer held).
    ///
    /// This method queries Redis to determine if the key associated with the lock
    /// is still present and matches the value of this lock. If the key is missing
    /// or the value does not match, the lock is considered freed.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the lock is considered freed (either because the key does not exist
    /// or the value does not match), otherwise `Ok(false)`. Returns an error if a Redis
    /// connection or query fails.
    pub async fn is_freed(&self, lock: &Lock) -> Result<bool, LockError> {
        match self.query_redis_for_key_value(&lock.resource).await? {
            Some(val) => {
                if val != lock.val {
                    Err(LockError::RedisKeyMismatch)
                } else {
                    Ok(false) // Key is present and matches the lock value
                }
            }
            None => Err(LockError::RedisKeyNotFound), // Key does not exist
        }
    }
}

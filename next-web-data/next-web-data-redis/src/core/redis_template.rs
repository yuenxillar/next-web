use std::{
    error::Error,
    ops::Deref,
    sync::{Arc, Mutex},
    time::Duration,
};

use redis::{AsyncConnectionConfig, Client, RedisResult, aio::MultiplexedConnection};

use crate::autoconfigure::redis_properties::RedisProperties;

#[cfg(feature = "distributed-lock")]
use crate::lock::{
    backend::{BlockingLockBackend, LockBackend, lock_is_gone},
    error::RedisLockError,
    protocol::{LockRequest, ScriptReply, commands::status},
};

/// Default Redis service exposed by the starter.
///
/// It wraps a `redis::Client` together with a small pool of multiplexed
/// connections so application code can resolve and use Redis immediately.
#[derive(Clone)]
pub struct RedisTemplate {
    /// Underlying Redis client.
    client: Arc<Client>,
    /// Redis async connection configuration.
    connection_config: Option<Arc<AsyncConnectionConfig>>,
    /// Connection the lock scripts run on, created on first use.
    ///
    /// A multiplexed connection supports concurrent requests, so one cached
    /// connection serves every async lock without a pool.
    #[cfg(feature = "distributed-lock")]
    lock_connections: Arc<tokio::sync::OnceCell<MultiplexedConnection>>,
    /// Connection the blocking lock scripts run on, created on first use.
    ///
    /// `redis::Connection` is `Send` but not `Sync`, so it is guarded by a
    /// mutex. The blocking backend holds it only for the duration of a command.
    #[cfg(feature = "distributed-lock")]
    blocking_lock_connection: Arc<Mutex<Option<redis::Connection>>>,
}

impl RedisTemplate {
    /// Create a new Redis template from a Redis URL.
    pub fn with_url<U>(url: U) -> Result<Self, Box<dyn Error>>
    where
        U: AsRef<str>,
    {
        let client = Arc::new(Client::open(url.as_ref())?);
        let connection_config = Default::default();

        Ok(Self {
            client,
            connection_config,
            #[cfg(feature = "distributed-lock")]
            lock_connections: Default::default(),
            #[cfg(feature = "distributed-lock")]
            blocking_lock_connection: Default::default(),
        })
    }

    /// Create a new Redis service from  RedisProperties.
    pub fn with_properties(properties: &RedisProperties) -> Result<Self, Box<dyn Error>> {
        let mut redis_template = Self::with_url(properties.to_url())?;

        let connection_config =
            if properties.connect_timeout().is_some() || properties.response_timeout().is_some() {
                let config = AsyncConnectionConfig::default()
                    .set_response_timeout(
                        properties
                            .response_timeout()
                            .map(|time| Duration::from_millis(time)),
                    )
                    .set_connection_timeout(
                        properties
                            .connect_timeout()
                            .map(|time| Duration::from_millis(time)),
                    );

                Arc::new(config).into()
            } else {
                Default::default()
            };

        redis_template.connection_config = connection_config;

        Ok(redis_template)
    }

    /// Return the underlying Redis client.
    pub fn client(&self) -> &Arc<Client> {
        &self.client
    }

    /// Return the next multiplexed connection using round-robin selection.
    pub async fn get_connection(&self) -> RedisResult<MultiplexedConnection> {
        if let Some(config) = self.connection_config.as_ref() {
            self.client
                .get_multiplexed_async_connection_with_config(config)
                .await
        } else {
            self.client.get_multiplexed_async_connection().await
        }
    }

    /// Return the cached connection the lock scripts run on.
    ///
    /// The first caller creates the connection; concurrent callers share it.
    /// A multiplexed connection multiplexes in-flight requests over one socket,
    /// which is exactly what a lock command needs.
    #[cfg(feature = "distributed-lock")]
    async fn lock_connection(&self) -> Result<MultiplexedConnection, RedisLockError> {
        if let Some(connection) = self.lock_connections.get() {
            return Ok(connection.clone());
        }

        let connection = self.get_connection().await?;
        // Losing this race means another caller already installed an
        // equivalent connection; either one is usable.
        let _ = self.lock_connections.set(connection.clone());

        Ok(self.lock_connections.get().cloned().unwrap_or(connection))
    }

    /// Run `operation` against the cached blocking connection.
    ///
    /// A connection is dropped when a command fails, so the next call
    /// reconnects instead of reusing a socket the server already closed.
    #[cfg(feature = "distributed-lock")]
    fn with_blocking_connection<T>(
        &self,
        timeout: Duration,
        operation: impl FnOnce(&mut redis::Connection) -> redis::RedisResult<T>,
    ) -> Result<T, RedisLockError> {
        let mut guard = self
            .blocking_lock_connection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if guard.is_none() {
            *guard = Some(self.client.get_connection_with_timeout(timeout)?);
        }

        let outcome = {
            let connection = match guard.as_mut() {
                Some(connection) => connection,
                None => {
                    return Err(RedisLockError::InvalidConfiguration(
                        "the blocking lock connection is unavailable".to_string(),
                    ));
                }
            };

            connection.set_read_timeout(Some(timeout))?;

            operation(connection)
        };

        match outcome {
            Ok(value) => Ok(value),
            Err(error) => {
                *guard = None;
                Err(RedisLockError::Redis(error))
            }
        }
    }
}

impl Deref for RedisTemplate {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[cfg(feature = "distributed-lock")]
#[next_web_core::async_trait]
impl LockBackend for RedisTemplate {
    async fn eval(
        &self,
        request: &LockRequest,
        timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError> {
        let mut connection = self.lock_connection().await?;

        let value = tokio::time::timeout(timeout, request.invoke_async(&mut connection))
            .await
            .map_err(|_| RedisLockError::CommandTimeout {
                script: request.script().name(),
            })??;

        ScriptReply::parse(value)
    }

    async fn wait_for_unlock(
        &self,
        name: &str,
        channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError> {
        // Subscribe first: a release that happens between the existence check
        // and the wait would otherwise be missed and turn into a full timeout.
        let mut pubsub = self.client.get_async_pubsub().await?;
        pubsub.subscribe(channel).await?;

        // `RedisTemplate` implements both backend traits, so the call has to say
        // which one it means.
        if lock_is_gone(&<Self as LockBackend>::eval(self, &status(name, ""), timeout).await?) {
            return Ok(());
        }

        let mut messages = pubsub.on_message();
        let _ = tokio::time::timeout(timeout, futures::StreamExt::next(&mut messages)).await;

        Ok(())
    }
}

#[cfg(feature = "distributed-lock")]
impl BlockingLockBackend for RedisTemplate {
    fn eval(
        &self,
        request: &LockRequest,
        timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError> {
        let value = self.with_blocking_connection(timeout, |connection| {
            request.invoke(connection)
        })?;

        ScriptReply::parse(value)
    }

    fn wait_for_unlock(
        &self,
        name: &str,
        _channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError> {
        // A blocking caller has no subscription to wake it, so it polls. The
        // lock layer already bounded `timeout` by the retry interval.
        if lock_is_gone(&<Self as BlockingLockBackend>::eval(self, &status(name, ""), timeout)?) {
            return Ok(());
        }

        std::thread::sleep(timeout);

        Ok(())
    }
}

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
        // `with_url` only parses the URL, so this test needs no live server.
        let template = RedisTemplate::with_url("redis://127.0.0.1:6379/0").unwrap();

        // Two traits in scope define the same method name, so qualify the call.
        assert_eq!(
            <RedisTemplate as LockBackend>::publish_command(&template),
            "PUBLISH"
        );
        assert_eq!(
            <RedisTemplate as BlockingLockBackend>::publish_command(&template),
            "PUBLISH"
        );
        assert_eq!(
            status("orders", "c:1").keys(),
            ["orders".to_string()].as_slice()
        );
    }
}

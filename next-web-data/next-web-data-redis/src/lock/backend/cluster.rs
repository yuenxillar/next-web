//! Transport for a Redis Cluster deployment.
//!
//! Every lock key and its Pub/Sub channel share a hash tag, so a script that
//! touches both still runs on one node. Notification waiting degrades to
//! polling here: a RESP2 cluster subscription needs a node-addressed connection
//! of its own, and the correctness of a release never depends on the wakeup.

use std::{sync::Arc, time::Duration};

use next_web_core::async_trait;
use redis::cluster::ClusterClient;

use crate::lock::backend::{BlockingLockBackend, LockBackend, lock_is_gone};
use crate::lock::error::RedisLockError;
use crate::lock::protocol::{LockRequest, ScriptReply, commands::status};

/// Transport for a Redis Cluster deployment.
#[derive(Clone)]
pub struct ClusterLockBackend {
    client: Arc<ClusterClient>,
}

impl ClusterLockBackend {
    /// Build a backend from cluster seed URLs.
    ///
    /// `command_timeout` becomes the client's own response timeout, which is
    /// what bounds a blocking script call: a synchronous socket read cannot be
    /// cancelled from the outside the way a future can.
    pub fn new(urls: Vec<String>, command_timeout: Duration) -> Result<Self, RedisLockError> {
        if urls.is_empty() {
            return Err(RedisLockError::InvalidConfiguration(
                "cluster mode requires at least one Redis URL".to_string(),
            ));
        }

        let client = ClusterClient::builder(urls)
            .response_timeout(command_timeout)
            .build()?;

        Ok(Self {
            client: Arc::new(client),
        })
    }
}

#[async_trait]
impl LockBackend for ClusterLockBackend {
    async fn eval(
        &self,
        request: &LockRequest,
        timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError> {
        let mut connection = self.client.get_async_connection().await?;

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
        _channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError> {
        if lock_is_gone(&<Self as LockBackend>::eval(self, &status(name, ""), timeout).await?) {
            return Ok(());
        }

        tokio::time::sleep(timeout).await;

        Ok(())
    }

    fn publish_command(&self) -> &'static str {
        "SPUBLISH"
    }
}

impl BlockingLockBackend for ClusterLockBackend {
    fn eval(
        &self,
        request: &LockRequest,
        _timeout: Duration,
    ) -> Result<ScriptReply, RedisLockError> {
        // The client's response timeout bounds this call.
        let mut connection = self.client.get_connection()?;

        ScriptReply::parse(request.invoke(&mut connection)?)
    }

    fn wait_for_unlock(
        &self,
        name: &str,
        _channel: &str,
        timeout: Duration,
    ) -> Result<(), RedisLockError> {
        if lock_is_gone(&<Self as BlockingLockBackend>::eval(
            self,
            &status(name, ""),
            timeout,
        )?) {
            return Ok(());
        }

        std::thread::sleep(timeout);

        Ok(())
    }

    fn publish_command(&self) -> &'static str {
        "SPUBLISH"
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
        // `build` only validates the URLs, so this needs no live cluster.
        let backend = ClusterLockBackend::new(
            vec!["redis://127.0.0.1:7000".to_string()],
            Duration::from_secs(3),
        )
        .unwrap();

        assert_eq!(BlockingLockBackend::publish_command(&backend), "SPUBLISH");
        assert_eq!(LockBackend::publish_command(&backend), "SPUBLISH");
    }
}

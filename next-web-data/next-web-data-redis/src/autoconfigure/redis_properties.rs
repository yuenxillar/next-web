use next_web_macros::configuration_properties;

use crate::autoconfigure::{
    redis_cluster_properties::RedisClusterProperties, redis_lock_properties::RedisLockProperties,
    redis_sentinel_properties::RedisSentinelProperties,
};
#[cfg(feature = "distributed-lock")]
use crate::lock::RedisLockConfig;

/// Configuration properties for Redis.
///
/// These values are typically bound from the `next.data.redis` prefix in `application.yaml`.
#[configuration_properties(prefix = "next.data.redis", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct RedisProperties {
    /// Full Redis connection URL (e.g., `redis://user:password@host:port/database`).
    ///
    /// If provided, individual components (host, port, etc.) will override
    /// the corresponding values parsed from the URL.
    url: Option<String>,

    /// Redis server hostname or IP address.
    /// Defaults to `localhost` if neither `url` nor `host` is specified.
    host: Option<String>,

    /// Redis server port.
    /// Defaults to `6379` if neither `url` nor `port` is specified.
    port: Option<u16>,

    /// Username for Redis ACL authentication (Redis 6.0+).
    ///
    /// If omitted, only password-based authentication is attempted
    /// (compatible with Redis versions prior to 6.0).
    username: Option<String>,

    /// Password for Redis authentication.
    ///
    /// Corresponds to the `requirepass` setting or ACL user password.
    password: Option<String>,

    /// Redis database index to select after connecting.
    /// Valid range: `0` to `15` (configurable via `databases` in redis.conf).
    /// Defaults to `0` if not specified.
    database: Option<u16>,

    /// Maximum time (in milliseconds) to wait for establishing a TCP connection.
    ///
    /// If `None`, the client library's default timeout is used.
    connect_timeout: Option<u64>,

    /// Maximum time (in milliseconds) to wait for a Redis command response.
    ///
    /// If `None`, no timeout is enforced (the client will wait indefinitely).
    response_timeout: Option<u64>,

    /// Enable Redis RESP3 protocol (requires Redis 6.0+).
    ///
    /// RESP3 provides richer type semantics (e.g., native booleans, maps, doubles)
    /// and enables features like client-side caching.
    ///
    /// When `true`, the client will negotiate RESP3 during connection handshake.
    /// If the server does not support RESP3, it automatically falls back to RESP2.
    resp3: Option<bool>,

    /// Cluster properties for connecting to a Redis cluster.
    ///
    /// When set, the client bootstraps from the given cluster nodes and
    /// follows redirects across the cluster.
    cluster: Option<RedisClusterProperties>,

    /// Sentinel properties for connecting to Redis via Sentinel.
    ///
    /// When set, the client discovers the master (or replica) node through
    /// the configured Sentinel instances.
    sentinel: Option<RedisSentinelProperties>,

    /// Distributed lock configuration. Ignored unless the `distributed-lock`
    /// Cargo feature is enabled.
    #[cfg_attr(not(feature = "distributed-lock"), allow(dead_code))]
    #[serde(default)]
    lock: RedisLockProperties,
}

impl RedisProperties {
    pub fn to_url(&self) -> String {
        // redis://localhost:6379
        if let Some(url) = self.url().filter(|s| s.len() >= 22) {
            return url.to_string();
        }

        let auth = self
            .username
            .as_deref()
            .zip(self.password.as_deref())
            .map(|(u, p)| format!("{}:{}@", u, p))
            .or_else(|| self.password.as_deref().map(|p| format!(":{}@", p)))
            .unwrap_or_default();

        let params = [self.resp3.unwrap_or_default().then_some("protocol=resp3")]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("&");

        let query = if params.is_empty() {
            Default::default()
        } else {
            String::from("?") + &params
        };

        format!(
            "redis://{}{}:{}/{}{}",
            auth,
            self.host(),
            self.port(),
            self.database(),
            query
        )
    }
}

impl RedisProperties {
    /// Return the Redis URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Return the configured Redis username.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Return the configured Redis password.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Return the configured Redis host.
    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("localhost")
    }

    /// Return the configured Redis port.
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(6379)
    }

    /// Return the configured Redis database index.
    pub fn database(&self) -> u16 {
        self.database.unwrap_or_default()
    }

    /// Return the configured connection timeout in milliseconds.
    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    /// Return the configured response timeout in milliseconds.
    pub fn response_timeout(&self) -> Option<u64> {
        self.response_timeout
    }

    /// Return whether to use RESP3 protocol.
    /// Defaults to `false`.
    pub fn is_resp3(&self) -> bool {
        self.resp3.unwrap_or_default()
    }

    /// Returns the cluster properties, or `None` if not configured.
    pub fn cluster(&self) -> Option<&RedisClusterProperties> {
        self.cluster.as_ref()
    }

    /// Returns the sentinel properties, or `None` if not configured.
    pub fn sentinel(&self) -> Option<&RedisSentinelProperties> {
        self.sentinel.as_ref()
    }

    /// Whether distributed locks should be enabled.
    #[cfg(feature = "distributed-lock")]
    pub fn lock_enabled(&self) -> bool {
        self.lock.enabled()
    }

    /// Whether the lock deployment is a Redis Cluster rather than a standalone server.
    #[cfg(feature = "distributed-lock")]
    pub fn lock_uses_cluster(&self) -> bool {
        self.lock.uses_cluster()
    }

    /// The cluster seed URLs the locks should use.
    #[cfg(feature = "distributed-lock")]
    pub fn lock_cluster_urls(&self) -> &[String] {
        self.lock.cluster_urls()
    }

    /// The independent deployment URLs a quorum lock should use.
    ///
    /// Fewer than three entries means no quorum lock is configured.
    #[cfg(feature = "distributed-lock")]
    pub fn lock_redlock_urls(&self) -> &[String] {
        self.lock.redlock_urls()
    }

    /// The timing knobs the lock service is built with.
    ///
    /// # Errors
    ///
    /// Returns the offending duration text when one cannot be parsed.
    #[cfg(feature = "distributed-lock")]
    pub fn lock_config(&self) -> Result<RedisLockConfig, String> {
        self.lock.config()
    }

    /// Set the Redis URL.
    pub fn set_url(&mut self, url: impl Into<String>) {
        self.url = Some(url.into());
    }

    /// Set the Redis username.
    pub fn set_username(&mut self, username: impl Into<String>) {
        self.username = Some(username.into());
    }

    /// Set the Redis password.
    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = Some(password.into());
    }

    /// Set the Redis host.
    pub fn set_host(&mut self, host: impl Into<String>) {
        self.host = Some(host.into());
    }

    /// Set the Redis port.
    pub fn set_port(&mut self, port: u16) {
        self.port = Some(port);
    }

    /// Set the Redis database index.
    pub fn set_database(&mut self, database: u16) {
        self.database = Some(database);
    }

    /// Set the connection timeout in milliseconds.
    pub fn set_connect_timeout(&mut self, connect_timeout: u64) {
        self.connect_timeout = Some(connect_timeout);
    }

    /// Set the response timeout in milliseconds.
    pub fn set_response_timeout(&mut self, response_timeout: u64) {
        self.response_timeout = Some(response_timeout);
    }

    /// Set whether to use RESP3 protocol.
    pub fn set_resp3(&mut self, resp3: bool) {
        self.resp3 = Some(resp3);
    }

    /// Sets the cluster properties.
    ///
    /// Wraps the given properties in `Some` and stores them.
    pub fn set_cluster(&mut self, cluster: RedisClusterProperties) {
        self.cluster = Some(cluster)
    }

    /// Sets the sentinel properties.
    ///
    /// Wraps the given properties in `Some` and stores them.
    pub fn set_sentinel(&mut self, sentinel: RedisSentinelProperties) {
        self.sentinel = Some(sentinel)
    }
}

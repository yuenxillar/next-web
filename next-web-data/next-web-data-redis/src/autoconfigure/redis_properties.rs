use next_web_macros::properties;
use rudi_dev::singleton;

/// Configuration properties for the Redis starter.
///
/// These values are typically bound from the `next.data.redis` prefix in `application.yaml`.
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.data.redis", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct RedisProperties {
    /// Full Redis connection URL (e.g., `redis://[:password@]host:port[/database]`).
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
}

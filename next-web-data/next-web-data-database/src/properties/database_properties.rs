use next_web_macros::properties;
use rudi_dev::singleton;

/// Properties for Database client.
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.data.database")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct DatabaseClientProperties {
    /// Unique identifier for the database client.
    id: Option<String>,
    /// Database driver name.
    driver: String,
    /// Database connection Host.
    host: Option<String>,
    /// Database connection Port.
    port: Option<u16>,
    /// Database username.
    username: Option<String>,
    /// Database password.
    password: Option<String>,
    /// Database name.
    database: String,
    /// Database URL extra parameters.
    url_extra: Option<String>,
    /// Maximum pool size.
    max_connections: Option<u64>,
    /// Minimum idle connections.
    min_connections: Option<u64>,
    /// Database connect timeout in seconds.
    connect_timeout: Option<u64>,
    /// Idle timeout in seconds.
    idle_timeout: Option<u64>,
    /// Pool acquire timeout in seconds.
    acquire_timeout: Option<u64>,
}

impl DatabaseClientProperties {
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn driver(&self) -> &str {
        &self.driver
    }

    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port.clone()
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    pub fn url_extra(&self) -> Option<&str> {
        self.url_extra.as_ref().map(|s| s.as_str())
    }

    pub fn max_connections(&self) -> Option<u64> {
        self.max_connections
    }

    pub fn min_connections(&self) -> Option<u64> {
        self.min_connections
    }

    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    pub fn idle_timeout(&self) -> Option<u64> {
        self.idle_timeout
    }

    pub fn acquire_timeout(&self) -> Option<u64> {
        self.acquire_timeout
    }
}

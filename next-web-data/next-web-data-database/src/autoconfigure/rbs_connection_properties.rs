use next_web_macros::properties;
use rudi_dev::singleton;

#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.datasource.rbs")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct RbsConnectionProperties {
    /// Maximum pool size. (default: 32)
    max_connections: Option<u64>,

    /// Not used
    /// Minimum idle connections.
    min_connections: Option<u64>,

    /// Maximum idle connections (default: same as max_open)
    max_idle_connections: Option<u64>,

    /// Database connect timeout in seconds.
    connect_timeout: Option<u64>,

    /// Database connect timeout in seconds.
    timeout_check: Option<u64>,
}

impl RbsConnectionProperties {
    /// Returns the `max_connections` field.
    pub fn max_connections(&self) -> Option<u64> {
        self.max_connections
    }

    /// Returns the `min_connections` field.
    pub fn min_connections(&self) -> Option<u64> {
        self.min_connections
    }

    /// Returns the `max_idle_connections` field.
    pub fn max_idle_connections(&self) -> Option<u64> {
        self.max_idle_connections
    }

    /// Returns the `connect_timeout` field.
    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    /// Returns the `timeout_check` field.
    pub fn timeout_check(&self) -> Option<u64> {
        self.timeout_check
    }

    /// Sets the `max_connections` field.
    pub fn set_max_connections<V>(&mut self, value: V)
    where
        V: Into<Option<u64>>,
    {
        self.max_connections = value.into();
    }

    /// Sets the `min_connections` field.
    pub fn set_min_connections<V>(&mut self, value: V)
    where
        V: Into<Option<u64>>,
    {
        self.min_connections = value.into();
    }

    /// Sets the `max_idle_connections` field.
    pub fn set_max_idle_connections<V>(&mut self, value: V)
    where
        V: Into<Option<u64>>,
    {
        self.max_idle_connections = value.into();
    }

    /// Sets the `connect_timeout` field.
    pub fn set_connect_timeout<V>(&mut self, value: V)
    where
        V: Into<Option<u64>>,
    {
        self.connect_timeout = value.into();
    }

    /// Sets the `timeout_check` field.
    pub fn set_timeout_check<V>(&mut self, value: V)
    where
        V: Into<Option<u64>>,
    {
        self.timeout_check = value.into();
    }
}
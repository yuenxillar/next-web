use next_web_macros::Properties;
use rudi_dev::Singleton;

/// Properties for Redis client.
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.redis")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RedisClientProperties {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    database: Option<u64>,
    connect_timeout: Option<u64>,
}

impl RedisClientProperties {
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn database(&self) -> Option<u64> {
        self.database
    }

    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    pub fn set_password(&mut self, password: Option<String>) {
        self.password = password;
    }

    pub fn set_host(&mut self, host: Option<String>) {
        self.host = host;
    }

    pub fn set_port(&mut self, port: Option<u16>) {
        self.port = port;
    }

    pub fn set_database(&mut self, database: Option<u64>) {
        self.database = database;
    }

    pub fn set_connect_timeout(&mut self, connect_timeout: Option<u64>) {
        self.connect_timeout = connect_timeout;
    }
}

impl Default for RedisClientProperties {
    fn default() -> Self {
        Self {
            host: Some("localhost".into()),
            port: Some(6379),
            username: None,
            password: None,
            database: Some(0),
            connect_timeout: Some(5000),
        }
    }
}

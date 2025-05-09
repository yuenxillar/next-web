use rudi_dev::{Properties, Singleton};

/// Properties for Redis client.
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.redis")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
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

    pub fn host(&self) -> Option<String> {
        self.host.clone()
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
}

// impl Default for RedisClientProperties {
//     fn default() -> Self {
//         Self {
//             host: Some("localhost".into()),
//             port: Some(6379),
//             username: None,
//             password: None,
//             database: None,
//             connect_timeout: Some(5000),
//         }
//     }
// }

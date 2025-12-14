use next_web_macros::Properties;
use rudi_dev::Singleton;

/// Properties for Mongod client.
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.mongodb")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MongodbClientProperties {
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    connect_timeout: Option<u64>,
    zstd: bool,
}

impl MongodbClientProperties {
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

    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }
    
    pub fn zstd(&self) -> bool {
        self.zstd
    }
}


impl Default for MongodbClientProperties {
    fn default() -> Self {
        Self {
            username: Some("root".into()),
            password: None, 
            host: Some("localhost".into()),
            port: Some(27017),
            database: None,
            connect_timeout: Some(5000),
            zstd: false,
        }
    }
}
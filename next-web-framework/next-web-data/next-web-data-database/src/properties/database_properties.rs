use rudi_dev::{Properties, Singleton};

/// Properties for Database client.
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.database")]
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
}

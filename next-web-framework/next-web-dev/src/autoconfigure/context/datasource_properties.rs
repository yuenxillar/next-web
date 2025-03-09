#[derive(Debug, Clone, serde::Deserialize)]
pub struct DataSourceProperties {
    id: String,
    driver: String,
    host: String,
    port: u16,
    username: String,
    password: String,
    database: String,
    url_extra: Option<String>,
}

impl DataSourceProperties {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn driver(&self) -> &str {
        &self.driver
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    pub fn url_extra(&self) -> Option<&str> {
        self.url_extra.as_ref().map(|s| s.as_str())
    }
}

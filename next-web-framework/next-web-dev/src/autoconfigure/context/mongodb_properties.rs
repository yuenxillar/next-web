#[derive(Debug, serde::Deserialize, Clone)]
pub struct MongoDBProperties {
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    zstd: bool,
}

impl MongoDBProperties {
    
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

    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    pub fn zstd(&self) -> bool {
        self.zstd
    }
}
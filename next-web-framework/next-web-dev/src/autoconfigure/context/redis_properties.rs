#[derive(Debug, Clone, serde::Deserialize)]
pub struct RedisProperties {
    host: String,
    port: u16,
    database: u8,
    password: Option<String>,
    timeout: Option<u64>,
}

impl RedisProperties {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn database(&self) -> u8 {
        self.database
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn timeout(&self) -> Option<u64> {
        self.timeout
    }
}

impl Default for RedisProperties {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6379,
            database: 0,
            password: None,
            timeout: None,
        }
    }
}

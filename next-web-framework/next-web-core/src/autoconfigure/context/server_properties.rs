use serde::Deserialize;

use super::http_properties::HttpProperties;

/// Application server register
#[derive(Debug, Deserialize, Clone)]
pub struct ServerProperties {
    port: u16,
    context_path: String,
    http: Option<HttpProperties>,
}

impl ServerProperties {
    pub fn new(port: u16, context_path: String, http: Option<HttpProperties>) -> Self {
        Self {
            port,
            context_path,
            http,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn context_path(&self) -> &str {
        &self.context_path
    }

    pub fn http(&self) -> Option<&HttpProperties> {
        self.http.as_ref()
    }
}

impl Default for ServerProperties {
    fn default() -> Self {
        ServerProperties {
            port: 10001,
            context_path: String::new(),
            http: None,
        }
    }
}

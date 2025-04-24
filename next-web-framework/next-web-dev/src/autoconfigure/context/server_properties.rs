use serde::Deserialize;

use super::http_properties::HttpProperties;

/// Application server register
#[derive(Debug, Deserialize, Clone)]
pub struct ServerProperties {
    port: Option<u16>,
    context_path: String,
    http: Option<HttpProperties>,
    local: Option<bool>
}

impl ServerProperties {
    pub fn new(port: Option<u16>, context_path: String, http: Option<HttpProperties>) -> Self {
        Self {
            port,
            context_path,
            http,
            local: None
        }
    }

    pub fn port(&self) -> Option<u16> {
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
            port: Some(63018),
            context_path: String::new(),
            http: None,
            local: None
        }
    }
}

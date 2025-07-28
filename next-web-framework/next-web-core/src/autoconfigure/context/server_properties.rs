use serde::Deserialize;

use crate::constants::application_constants::APPLICATION_DEFAULT_PORT;

use super::http_properties::HttpProperties;

/// Application server register
#[derive(Debug, Default, Deserialize, Clone)]
pub struct ServerProperties {
    #[serde(default = "default_port")]
    port: Option<u16>,
    context_path: Option<String>,
    http: Option<HttpProperties>,
    local: Option<bool>,
}

impl ServerProperties {
    pub fn new(
        port: Option<u16>,
        context_path: Option<String>,
        http: Option<HttpProperties>,
        local: Option<bool>,
    ) -> Self {
        Self {
            port,
            context_path,
            http,
            local,
        }
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn context_path(&self) -> Option<&str> {
        self.context_path.as_deref()
    }

    pub fn http(&self) -> Option<&HttpProperties> {
        self.http.as_ref()
    }

    pub fn local(&self) -> Option<bool> {
        self.local
    }

}

fn default_port() -> Option<u16> { Some(APPLICATION_DEFAULT_PORT)}
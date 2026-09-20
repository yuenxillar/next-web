use next_web_core::constants::application_constants::APPLICATION_DEFAULT_PORT;
use serde::Deserialize;

use std::sync::OnceLock;

/// Global server properties
pub static GLOBAL_SERVER_PROPERTIES: OnceLock<ServerProperties> = OnceLock::new();

/// Application server register
#[derive(Debug, Clone, Deserialize)]
pub struct ServerProperties {
    #[serde(default = "default_port")]
    port: u16,
    address: Option<String>,
    context_path: Option<String>,

    #[serde(default = "default_local")]
    local: bool,
}

impl ServerProperties {
    pub fn new(port: u16, address: String, context_path: String, local: bool) -> Self {
        Self {
            port,
            address: Some(address),
            context_path: Some(context_path),
            local,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    pub fn context_path(&self) -> Option<&str> {
        self.context_path.as_deref()
    }

    pub fn local(&self) -> bool {
        self.local
    }
}

fn default_port() -> u16 {
    APPLICATION_DEFAULT_PORT
}

fn default_local() -> bool {
    true
}

impl Default for ServerProperties {
    fn default() -> Self {
        Self {
            port: default_port(),
            address: None,
            context_path: None,
            local: true,
        }
    }
}

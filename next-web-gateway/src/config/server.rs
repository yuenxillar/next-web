//! Server and admin listener configuration.

use std::time::Duration;

use serde::Deserialize;

use super::duration::deserialize as deserialize_duration;

/// Anonymous defaults used when a field is omitted from the configuration.
fn default_bind() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_admin_bind() -> String {
    "127.0.0.1:9001".to_string()
}

fn default_graceful_shutdown() -> Duration {
    Duration::from_secs(30)
}

fn default_keepalive() -> Duration {
    Duration::from_secs(60)
}

/// Downstream listener settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Address the proxy listener binds to.
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Number of worker threads; `0` lets pingora decide.
    #[serde(default)]
    pub threads: usize,
    /// How long in-flight requests may finish after a shutdown signal.
    #[serde(default = "default_graceful_shutdown", deserialize_with = "deserialize_duration")]
    pub graceful_shutdown: Duration,
    /// Downstream keep-alive timeout.
    #[serde(default = "default_keepalive", deserialize_with = "deserialize_duration")]
    pub keepalive: Duration,
    /// Maximum concurrent downstream connections; `0` means unlimited.
    #[serde(default)]
    pub max_connection: usize,
    /// Admin listener settings.
    #[serde(default)]
    pub admin: AdminConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            threads: 0,
            graceful_shutdown: default_graceful_shutdown(),
            keepalive: default_keepalive(),
            max_connection: 0,
            admin: AdminConfig::default(),
        }
    }
}

/// Admin listener for health checks, metrics and route introspection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminConfig {
    /// Whether the admin listener is started at all.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Address the admin listener binds to.
    #[serde(default = "default_admin_bind")]
    pub bind: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind: default_admin_bind(),
        }
    }
}

fn default_true() -> bool {
    true
}

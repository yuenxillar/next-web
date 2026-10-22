//! Server and admin listener configuration.

use std::time::Duration;

use serde::Deserialize;

use super::duration::{
    deserialize as deserialize_duration, deserialize_option as deserialize_optional_duration,
};

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

/// Downstream listener settings.
///
/// Only the keys that can actually be applied are accepted: `threads` and
/// `graceful_shutdown` map onto pingora's `ServerConf`, while `keepalive` and
/// `max_connection` are rejected at startup because pingora 0.9 exposes no
/// downstream knob for them. Accepting them would be a silent no-op.
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
    ///
    /// Not supported: rejected at startup when present.
    #[serde(default, deserialize_with = "deserialize_optional_duration")]
    pub keepalive: Option<Duration>,
    /// Maximum concurrent downstream connections; `0` means unlimited.
    ///
    /// Not supported: any non-zero value is rejected at startup.
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
            keepalive: None,
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

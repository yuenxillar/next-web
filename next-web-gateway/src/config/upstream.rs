//! Upstream defaults: timeouts, health checks and TLS trust.

use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use super::duration::deserialize as deserialize_duration;

/// Defaults applied to every upstream connection.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamConfig {
    /// TCP connect timeout.
    #[serde(
        default = "default_connect_timeout",
        deserialize_with = "deserialize_duration"
    )]
    pub connect_timeout: Duration,
    /// Time allowed to read the upstream response.
    #[serde(
        default = "default_read_timeout",
        deserialize_with = "deserialize_duration"
    )]
    pub read_timeout: Duration,
    /// Time allowed to write the upstream request.
    #[serde(
        default = "default_write_timeout",
        deserialize_with = "deserialize_duration"
    )]
    pub write_timeout: Duration,
    /// Idle connection reuse timeout.
    #[serde(
        default = "default_idle_timeout",
        deserialize_with = "deserialize_duration"
    )]
    pub idle_timeout: Duration,
    /// Passive health checking for `lb://` upstreams.
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    /// TLS trust settings for `https://` upstreams.
    #[serde(default)]
    pub tls: TlsConfig,
}

impl Default for UpstreamConfig {
    fn default() -> Self {
        Self {
            connect_timeout: default_connect_timeout(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
            idle_timeout: default_idle_timeout(),
            health_check: HealthCheckConfig::default(),
            tls: TlsConfig::default(),
        }
    }
}

fn default_connect_timeout() -> Duration {
    Duration::from_secs(2)
}

fn default_read_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_write_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_idle_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_health_interval() -> Duration {
    Duration::from_secs(5)
}

fn default_true() -> bool {
    true
}

/// TCP health checking of `lb://` backends.
///
/// Not implemented in this build: `enabled: true` is rejected at startup so an
/// operator never believes dead backends are being drained when they are not.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthCheckConfig {
    /// Whether health checks run at all.
    #[serde(default)]
    pub enabled: bool,
    /// Interval between two health check rounds.
    #[serde(
        default = "default_health_interval",
        deserialize_with = "deserialize_duration"
    )]
    pub interval: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval: default_health_interval(),
        }
    }
}

/// TLS trust settings applied to `https://` upstream peers.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    /// Verify the upstream certificate chain against the trust store.
    #[serde(default = "default_true")]
    pub verify_cert: bool,
    /// Verify the upstream certificate matches the SNI host name.
    #[serde(default = "default_true")]
    pub verify_hostname: bool,
    /// Extra PEM bundle trusted in addition to the system roots.
    #[serde(default)]
    pub ca_file: Option<PathBuf>,
    /// Disable both certificate and host name verification.
    #[serde(default)]
    pub insecure_skip_verify: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_cert: true,
            verify_hostname: true,
            ca_file: None,
            insecure_skip_verify: false,
        }
    }
}

//! Configuration bound from `next.data.redis.lock`.
//!
//! This module owns the lock block end to end: the YAML shape, the duration
//! syntax, the topology choice, and the mapping to [`RedisLockConfig`]. Nothing
//! outside it reads the fields, so the block can move or grow without touching
//! the rest of the properties.

#[cfg(feature = "distributed-lock")]
use crate::lock::RedisLockConfig;

/// Configuration bound from `next.data.redis.lock`.
///
/// Keys are kebab-case, matching the rest of the starter's configuration:
/// `watchdog-timeout`, `retry-interval`, `command-timeout`.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RedisLockProperties {
    enabled: bool,
    mode: RedisLockPropertyMode,
    watchdog_timeout: Option<DurationValue>,
    retry_interval: Option<DurationValue>,
    command_timeout: Option<DurationValue>,
    cluster: RedisLockClusterProperties,
    redlock: RedisRedLockProperties,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct RedisLockClusterProperties {
    urls: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct RedisRedLockProperties {
    urls: Vec<String>,
}

/// A duration as written in `application.yaml`: a bare millisecond count or a
/// string such as `30s`.
///
/// Only `config()` reads it, and that is feature-gated, so the type is unused in
/// a build without the feature.
#[cfg_attr(not(feature = "distributed-lock"), allow(dead_code))]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
enum DurationValue {
    Millis(u64),
    Text(String),
}

#[cfg(feature = "distributed-lock")]
impl DurationValue {
    /// The duration in milliseconds, accepting `100ms`, `30s`, `2m` or a bare number.
    fn millis(&self) -> Result<u64, String> {
        match self {
            Self::Millis(value) => Ok(*value),
            Self::Text(value) => parse_duration_millis(value),
        }
    }
}

#[cfg(feature = "distributed-lock")]
fn parse_duration_millis(value: &str) -> Result<u64, String> {
    let value = value.trim();
    let (number, multiplier) = if let Some(number) = value.strip_suffix("ms") {
        (number, 1)
    } else if let Some(number) = value.strip_suffix('s') {
        (number, 1_000)
    } else if let Some(number) = value.strip_suffix('m') {
        (number, 60_000)
    } else {
        (value, 1)
    };
    number
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("invalid duration: {value}"))?
        .checked_mul(multiplier)
        .ok_or_else(|| format!("duration is too large: {value}"))
}

/// Which deployment the locks run on.
#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedisLockPropertyMode {
    /// A standalone server or a single sentinel-managed master.
    #[default]
    Direct,
    /// A Redis Cluster.
    Cluster,
}

impl RedisLockProperties {
    /// Whether distributed locks should be enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether the lock deployment is a Redis Cluster rather than a standalone server.
    pub fn uses_cluster(&self) -> bool {
        matches!(self.mode, RedisLockPropertyMode::Cluster)
    }

    /// The cluster seed URLs the locks should use.
    pub fn cluster_urls(&self) -> &[String] {
        &self.cluster.urls
    }

    /// The independent deployment URLs a quorum lock should use.
    ///
    /// Fewer than three entries means no quorum lock is configured.
    pub fn redlock_urls(&self) -> &[String] {
        &self.redlock.urls
    }

    /// The timing knobs the lock service is built with.
    ///
    /// Topology is not part of the returned value: it decides which constructor
    /// the auto-configuration calls.
    ///
    /// # Errors
    ///
    /// Returns the offending duration text when one cannot be parsed, so a typo in
    /// `application.yaml` fails the startup instead of silently using a default.
    #[cfg(feature = "distributed-lock")]
    pub fn config(&self) -> Result<RedisLockConfig, String> {
        let mut config = RedisLockConfig::default();
        if let Some(value) = &self.watchdog_timeout {
            config.watchdog_timeout = std::time::Duration::from_millis(value.millis()?);
        }
        if let Some(value) = &self.retry_interval {
            config.retry_interval = std::time::Duration::from_millis(value.millis()?);
        }
        if let Some(value) = &self.command_timeout {
            config.command_timeout = std::time::Duration::from_millis(value.millis()?);
        }

        Ok(config)
    }
}

#[cfg(all(test, feature = "distributed-lock"))]
mod lock_tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn lock_duration_accepts_human_units() {
        assert_eq!(parse_duration_millis("100ms").unwrap(), 100);
        assert_eq!(parse_duration_millis("30s").unwrap(), 30_000);
        assert_eq!(parse_duration_millis("2m").unwrap(), 120_000);
        assert!(parse_duration_millis("soon").is_err());
    }

    #[test]
    fn the_yaml_block_binds_kebab_case_keys_and_maps_to_a_config() {
        // The keys are kebab-case; `rename_all` is what makes this bind at all,
        // and losing it would silently fall back to every default.
        let properties: RedisLockProperties = serde_yaml::from_str(
            r#"
enabled: true
mode: cluster
watchdog-timeout: 5s
retry-interval: 200ms
command-timeout: 1s
cluster:
  urls:
    - redis://127.0.0.1:7000
    - redis://127.0.0.1:7001
redlock:
  urls:
    - redis://one:6379
    - redis://two:6379
    - redis://three:6379
"#,
        )
        .expect("a documented lock block must deserialize");

        assert!(properties.enabled());
        assert!(properties.uses_cluster());
        assert_eq!(properties.cluster_urls().len(), 2);
        assert_eq!(properties.redlock_urls().len(), 3);

        let config = properties.config().unwrap();
        assert_eq!(config.watchdog_timeout, Duration::from_secs(5));
        assert_eq!(config.retry_interval, Duration::from_millis(200));
        assert_eq!(config.command_timeout, Duration::from_secs(1));
    }

    #[test]
    fn an_empty_block_keeps_every_default() {
        let properties: RedisLockProperties = serde_yaml::from_str("{}").unwrap();

        assert!(!properties.enabled());
        assert!(!properties.uses_cluster());
        assert!(properties.cluster_urls().is_empty());
        assert!(properties.redlock_urls().is_empty());

        let config = properties.config().unwrap();
        assert_eq!(
            config.watchdog_timeout,
            RedisLockConfig::default().watchdog_timeout
        );
        assert_eq!(
            config.retry_interval,
            RedisLockConfig::default().retry_interval
        );
        assert_eq!(
            config.command_timeout,
            RedisLockConfig::default().command_timeout
        );
    }
}

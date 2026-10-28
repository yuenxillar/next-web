//! Route, upstream entry and circuit breaker definitions.

use std::time::Duration;

use serde::Deserialize;

use super::duration::deserialize_option as deserialize_optional_duration;

/// One route: predicates select it, filters rewrite it, `uri` names its target.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteConfig {
    /// Unique route identifier, also used as the metrics and log label.
    pub id: String,
    /// Upstream target, one of `http://host:port`, `https://host:port` or `lb://name`.
    pub uri: String,
    /// Evaluation order, lower runs first; defaults to `i32::MAX`.
    #[serde(default = "default_order")]
    pub order: i32,
    /// Predicate expressions, all of which must match.
    #[serde(default)]
    pub predicates: Vec<String>,
    /// Filter expressions, applied in declaration order.
    #[serde(default)]
    pub filters: Vec<String>,
    /// Static backends for `lb://` routes.
    #[serde(default)]
    pub upstreams: Vec<UpstreamConfigEntry>,
    /// Optional per-route metadata.
    #[serde(default)]
    pub metadata: RouteMetadata,
}

fn default_order() -> i32 {
    i32::MAX
}

/// One static backend of an `lb://` route.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpstreamConfigEntry {
    /// `host:port` of the backend.
    pub addr: String,
    /// Relative weight used by the load balancer.
    #[serde(default = "default_weight")]
    pub weight: usize,
}

fn default_weight() -> usize {
    1
}

/// Per-route metadata that is not part of the predicate/filter DSL.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteMetadata {
    /// Per-route upstream timeouts, overriding the gateway defaults.
    #[serde(default)]
    pub client: ClientMetadata,
}

/// Per-route upstream timeout overrides.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientMetadata {
    /// TCP connect timeout override.
    #[serde(default, deserialize_with = "deserialize_optional_duration")]
    pub connect_timeout: Option<Duration>,
    /// Read timeout override.
    #[serde(default, deserialize_with = "deserialize_optional_duration")]
    pub read_timeout: Option<Duration>,
    /// Write timeout override.
    #[serde(default, deserialize_with = "deserialize_optional_duration")]
    pub write_timeout: Option<Duration>,
}

/// A circuit breaker, shared by every route that references its id.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BreakerConfig {
    /// Whether the breaker may open at all.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Consecutive failures that open the breaker.
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    /// How long the breaker stays open before probing again.
    #[serde(
        default = "default_wait_duration",
        deserialize_with = "super::duration::deserialize"
    )]
    pub wait_duration_in_open_state: Duration,
    /// Concurrent probes allowed while half open.
    #[serde(default = "default_half_open_permits")]
    pub half_open_permits: usize,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: default_failure_threshold(),
            wait_duration_in_open_state: default_wait_duration(),
            half_open_permits: default_half_open_permits(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_failure_threshold() -> u32 {
    10
}

fn default_wait_duration() -> Duration {
    Duration::from_secs(5)
}

fn default_half_open_permits() -> usize {
    1
}

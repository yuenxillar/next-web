//! Gateway configuration: schema, duration parsing and collected validation.
//!
//! The configuration is read from the `next.gateway` block. A legacy top level
//! `gateway` block is still accepted, with a deprecation warning. Unknown keys
//! are rejected so a typo cannot silently disable a filter.
//!
//! ```yaml
//! next:
//!   gateway:
//!     routes:
//!       - id: users
//!         uri: lb://user-service
//!         predicates: ["Path=/api/users/**"]
//!         filters: ["StripPrefix=1"]
//!         upstreams:
//!           - addr: "10.0.0.11:8080"
//! ```

mod duration;
mod error_model;
mod logging;
mod route;
mod server;
mod upstream;
mod validate;

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use serde::Deserialize;

pub use duration::parse_duration;
pub use error_model::{ErrorConfig, ErrorFormat};
pub use logging::{LogFormat, LoggingConfig};
pub use route::{BreakerConfig, ClientMetadata, RouteConfig, RouteMetadata, UpstreamConfigEntry};
pub use server::{AdminConfig, ServerConfig};
pub use upstream::{HealthCheckConfig, TlsConfig, UpstreamConfig};
pub use validate::is_valid_bind;

/// A single configuration problem, addressed by a dotted path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// Document path of the offending value, for example `routes[2].filters[1]`.
    pub path: String,
    /// Human readable description of the problem.
    pub message: String,
}

impl ConfigError {
    /// Build a configuration error from a path and a message.
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

/// Every problem found while loading a configuration, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigErrors {
    errors: Vec<ConfigError>,
}

impl ConfigErrors {
    /// Build the error set from collected failures.
    pub fn new(errors: Vec<ConfigError>) -> Self {
        Self { errors }
    }

    /// Borrow the collected failures.
    pub fn errors(&self) -> &[ConfigError] {
        &self.errors
    }

    /// Number of collected failures.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Whether nothing failed.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl fmt::Display for ConfigErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.errors.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigErrors {}

/// The complete gateway configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayConfig {
    /// Downstream listener settings.
    #[serde(default)]
    pub server: ServerConfig,
    /// Logging and access-log settings.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Upstream defaults.
    #[serde(default)]
    pub upstream: UpstreamConfig,
    /// Error response shape.
    #[serde(default)]
    pub error: ErrorConfig,
    /// Networks whose `X-Forwarded-For` entries are trusted. Empty trusts nobody.
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    /// Circuit breakers, referenced by id from the `CircuitBreaker` filter.
    #[serde(default)]
    pub breakers: HashMap<String, BreakerConfig>,
    /// Routes, evaluated in `order` then `id` order.
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
}

/// Top level document: the gateway block may sit under `next` or at the root.
#[derive(Debug, Default, Deserialize)]
struct RootFile {
    next: Option<NextNamespace>,
    gateway: Option<GatewayConfig>,
}

/// The `next` namespace, which also holds configuration of other crates.
#[derive(Debug, Default, Deserialize)]
struct NextNamespace {
    gateway: Option<GatewayConfig>,
}

impl GatewayConfig {
    /// Read and parse a configuration file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigErrors> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|error| {
            ConfigErrors::new(vec![ConfigError::new(
                "",
                format!("failed to read `{}`: {error}", path.display()),
            )])
        })?;

        Self::from_str(&text)
    }

    /// Graceful shutdown budget for the proxy listener.
    pub fn graceful_shutdown(&self) -> Duration {
        self.server.graceful_shutdown
    }

    /// Whether any route enables the in-process response cache.
    pub fn local_response_cache_enabled(&self) -> bool {
        self.routes.iter().any(|route| {
            route
                .filters
                .iter()
                .any(|filter| filter.starts_with("LocalResponseCache"))
        })
    }
}

impl std::str::FromStr for GatewayConfig {
    type Err = ConfigErrors;

    /// Parse a configuration document.
    fn from_str(text: &str) -> Result<Self, ConfigErrors> {
        let root: RootFile =
            match serde_path_to_error::deserialize(serde_yaml::Deserializer::from_str(text)) {
                Ok(root) => root,
                Err(error) => {
                    return Err(ConfigErrors::new(vec![ConfigError::new(
                        error.path().to_string(),
                        error.inner().to_string(),
                    )]));
                }
            };

        let config = match (root.next.and_then(|next| next.gateway), root.gateway) {
            (Some(_), Some(_)) => {
                return Err(ConfigErrors::new(vec![ConfigError::new(
                    "next.gateway",
                    "the gateway block is declared twice: use either the `next.gateway` \
                     block or the legacy top level `gateway` block",
                )]));
            }
            (Some(config), None) => config,
            (None, Some(config)) => {
                tracing::warn!(
                    "the top level `gateway` block is deprecated, nest it under `next.gateway`"
                );
                config
            }
            (None, None) => {
                return Err(ConfigErrors::new(vec![ConfigError::new(
                    "",
                    "no gateway configuration found: expected `next.gateway`",
                )]));
            }
        };

        let errors = validate::validate(&config);
        if errors.is_empty() {
            Ok(config)
        } else {
            Err(ConfigErrors::new(errors))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{ConfigError, GatewayConfig, parse_duration};

    #[test]
    fn parses_duration_units() {
        assert_eq!(parse_duration("500ms").unwrap().as_millis(), 500);
        assert_eq!(parse_duration("30s").unwrap().as_secs(), 30);
        assert_eq!(parse_duration("2m").unwrap().as_secs(), 120);
        assert_eq!(parse_duration("1h").unwrap().as_secs(), 3600);
        assert!(parse_duration("3 parsecs").is_err());
    }

    #[test]
    fn rejects_unknown_fields_with_path() {
        let err = GatewayConfig::from_str("next:\n  gateway:\n    bogus: 1\n").unwrap_err();
        assert!(err.to_string().contains("bogus"), "{err}");
    }

    #[test]
    fn accepts_both_root_keys_and_rejects_both_together() {
        assert!(GatewayConfig::from_str("next:\n  gateway:\n    routes: []\n").is_ok());
        assert!(GatewayConfig::from_str("gateway:\n  routes: []\n").is_ok());
        assert!(
            GatewayConfig::from_str("next:\n  gateway:\n    routes: []\ngateway:\n  routes: []\n")
                .is_err()
        );
    }

    #[test]
    fn reports_all_validation_errors_at_once() {
        let yaml = "next:\n  gateway:\n    routes:\n      - id: a\n        uri: nope\n      - id: a\n        uri: lb://x\n";
        let err = GatewayConfig::from_str(yaml).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("routes[0].uri"), "{text}");
        assert!(text.contains("routes[1].id"), "{text}");
        assert!(text.contains("routes[1].upstreams"), "{text}");
        assert_eq!(err.len(), 3, "{text}");
    }

    #[test]
    fn applies_documented_defaults() {
        let config = GatewayConfig::from_str("next:\n  gateway:\n    routes: []\n").unwrap();
        assert_eq!(config.server.bind, "0.0.0.0:8080");
        assert_eq!(config.server.admin.bind, "127.0.0.1:9001");
        assert_eq!(config.upstream.connect_timeout.as_secs(), 2);
        assert_eq!(config.upstream.read_timeout.as_secs(), 30);
        assert!(config.upstream.tls.verify_cert);
        assert!(config.upstream.tls.verify_hostname);
        assert!(config.trusted_proxies.is_empty());
        assert!(config.breakers.is_empty());
    }

    #[test]
    fn config_error_displays_path_and_message() {
        let error = ConfigError::new("routes[0].uri", "bad scheme");
        assert_eq!(error.to_string(), "routes[0].uri: bad scheme");
    }

    fn errors_for(yaml: &str) -> String {
        GatewayConfig::from_str(yaml).unwrap_err().to_string()
    }

    #[test]
    fn rejects_unknown_scheme() {
        let text = errors_for(
            "next:\n  gateway:\n    routes:\n      - id: a\n        uri: ftp://host:21\n",
        );
        assert!(text.contains("routes[0].uri"), "{text}");
        assert!(text.contains("ftp"), "{text}");
    }

    #[test]
    fn rejects_uri_without_authority() {
        let text =
            errors_for("next:\n  gateway:\n    routes:\n      - id: a\n        uri: 'http://'\n");
        assert!(text.contains("routes[0].uri"), "{text}");
    }

    #[test]
    fn rejects_an_authority_without_a_host() {
        let text = errors_for(
            "next:\n  gateway:\n    routes:\n      - id: a\n        uri: 'http://:8080'\n",
        );
        assert!(text.contains("routes[0].uri"), "{text}");
        assert!(text.contains("host"), "{text}");
    }

    #[test]
    fn accepts_an_https_route_now_that_a_tls_backend_is_compiled_in() {
        use std::str::FromStr;

        let yaml = "next:\n  gateway:\n    routes:\n      - id: a\n        uri: 'https://example.com'\n        predicates:\n          - Path=/**\n";
        let config = GatewayConfig::from_str(yaml).expect("https routes are supported");

        assert_eq!(config.routes[0].uri, "https://example.com");
    }

    #[test]
    fn health_checking_is_on_by_default_and_can_be_turned_off() {
        use std::str::FromStr;

        let default = GatewayConfig::from_str("next:\n  gateway:\n    routes: []\n").unwrap();
        assert!(default.upstream.health_check.enabled);

        let off = "next:\n  gateway:\n    upstream:\n      health_check:\n        enabled: false\n        interval: 2s\n";
        let config = GatewayConfig::from_str(off).expect("an explicit `enabled: false` is valid");

        assert!(!config.upstream.health_check.enabled);
        assert_eq!(
            config.upstream.health_check.interval,
            std::time::Duration::from_secs(2)
        );
    }

    #[test]
    fn accepts_the_tls_verification_controls() {
        use std::str::FromStr;

        let yaml = "next:\n  gateway:\n    upstream:\n      tls:\n        verify_cert: false\n        insecure_skip_verify: true\n";
        let config = GatewayConfig::from_str(yaml).expect("TLS settings are supported");

        assert!(config.upstream.tls.insecure_skip_verify);
    }

    #[test]
    fn rejects_invalid_port() {
        let text = errors_for(
            "next:\n  gateway:\n    routes:\n      - id: a\n        uri: 'http://host:notaport'\n",
        );
        assert!(text.contains("routes[0].uri"), "{text}");
    }

    #[test]
    fn rejects_upstreams_on_direct_routes() {
        let text = errors_for(
            "next:\n  gateway:\n    routes:\n      - id: a\n        uri: 'http://host:8080'\n        upstreams:\n          - addr: '1.1.1.1:80'\n",
        );
        assert!(text.contains("routes[0].upstreams"), "{text}");
    }

    #[test]
    fn rejects_invalid_trusted_proxy_cidr() {
        let text = errors_for("next:\n  gateway:\n    trusted_proxies: ['10.0.0.0/33']\n");
        assert!(text.contains("trusted_proxies[0]"), "{text}");
    }

    #[test]
    fn accepts_valid_trusted_proxy_entries() {
        let config = GatewayConfig::from_str(
            "next:\n  gateway:\n    trusted_proxies: ['10.0.0.0/8', '192.168.1.1']\n",
        )
        .unwrap();
        assert_eq!(config.trusted_proxies.len(), 2);
    }

    #[test]
    fn rejects_upstream_address_that_is_not_ip_port() {
        let text = errors_for(
            "next:\n  gateway:\n    routes:\n      - id: a\n        uri: lb://svc\n        upstreams:\n          - addr: 'svc.local:8080'\n",
        );
        assert!(text.contains("routes[0].upstreams[0].addr"), "{text}");
    }

    #[test]
    fn rejects_zero_upstream_weight() {
        let text = errors_for(
            "next:\n  gateway:\n    routes:\n      - id: a\n        uri: lb://svc\n        upstreams:\n          - addr: '10.0.0.1:8080'\n            weight: 0\n",
        );
        assert!(text.contains("routes[0].upstreams[0].weight"), "{text}");
    }

    #[test]
    fn rejects_invalid_bind_address() {
        let text = errors_for("next:\n  gateway:\n    server:\n      bind: 'localhost'\n");
        assert!(text.contains("server.bind"), "{text}");
    }

    #[test]
    fn accepts_hostname_bind_with_port() {
        let config = GatewayConfig::from_str(
            "next:\n  gateway:\n    server:\n      bind: 'localhost:8080'\n",
        )
        .unwrap();
        assert_eq!(config.server.bind, "localhost:8080");
    }

    #[test]
    fn rejects_zero_breaker_thresholds() {
        let text = errors_for(
            "next:\n  gateway:\n    breakers:\n      main:\n        failure_threshold: 0\n",
        );
        assert!(text.contains("breakers.main.failure_threshold"), "{text}");
    }

    #[test]
    fn rejects_unknown_route_field() {
        let text = errors_for(
            "next:\n  gateway:\n    routes:\n      - id: a\n        uri: 'http://h:80'\n        filter: ['StripPrefix=1']\n",
        );
        assert!(text.contains("filter"), "{text}");
    }

    #[test]
    fn applies_breaker_defaults() {
        let config = GatewayConfig::from_str(
            "next:\n  gateway:\n    breakers:\n      main:\n        failure_threshold: 3\n",
        )
        .unwrap();
        let breaker = &config.breakers["main"];
        assert_eq!(breaker.failure_threshold, 3);
        assert_eq!(breaker.wait_duration_in_open_state.as_secs(), 5);
        assert_eq!(breaker.half_open_permits, 1);
        assert!(breaker.enabled);
    }

    #[test]
    fn accepts_a_full_valid_configuration() {
        let yaml = "\
next:
  gateway:
    server:
      bind: '0.0.0.0:8080'
      graceful_shutdown: 30s
      admin:
        bind: '127.0.0.1:9001'
    logging:
      level: info
      format: json
      access_log: true
    upstream:
      connect_timeout: 1s
      read_timeout: 10s
      write_timeout: 10s
      idle_timeout: 60s
      tls:
        verify_cert: true
        ca_file: './ca.pem'
    error:
      format: text
      request_id_header: X-Trace-Id
    trusted_proxies: ['10.0.0.0/8']
    breakers:
      main:
        enabled: true
        failure_threshold: 5
        wait_duration_in_open_state: 8s
        half_open_permits: 2
    routes:
      - id: users
        uri: lb://user-service
        order: 100
        predicates: ['Path=/api/users/**']
        filters: ['StripPrefix=1']
        upstreams:
          - addr: '10.0.0.11:8080'
            weight: 2
        metadata:
          client:
            read_timeout: 5s
";
        let config = GatewayConfig::from_str(yaml).unwrap();
        assert_eq!(config.routes.len(), 1);
        assert_eq!(config.routes[0].order, 100);
        assert_eq!(config.routes[0].upstreams[0].weight, 2);
        assert_eq!(config.logging.format, super::LogFormat::Json);
        assert_eq!(config.error.format, super::ErrorFormat::Text);
        assert_eq!(
            config.routes[0]
                .metadata
                .client
                .read_timeout
                .unwrap()
                .as_secs(),
            5
        );
        assert_eq!(config.breakers["main"].half_open_permits, 2);
        assert_eq!(config.graceful_shutdown().as_secs(), 30);
    }
}

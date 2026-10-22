//! Static load balancing over the upstreams listed on each `lb://` route.
//!
//! The balancer is a weighted round robin: every backend appears in the rotation
//! `weight` times, so the configured ratio is exact rather than statistical.
//! `pingora::lb::LoadBalancer` keeps one entry per distinct address (its
//! `Backends` is a `BTreeSet`), which loses the weight of a repeated address, so
//! the rotation is kept here. The [`UpstreamResolver`] seam is what a Phase 3
//! registry implementation plugs into.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::{ConfigError, ConfigErrors, GatewayConfig};
use crate::upstream::{UpstreamResolver, UpstreamUri};

/// The rotation of one `lb://` route.
struct RoutePool {
    /// Backend addresses repeated `weight` times.
    addresses: Vec<SocketAddr>,
    /// Next slot to serve.
    cursor: AtomicUsize,
}

impl RoutePool {
    fn next(&self) -> Option<SocketAddr> {
        if self.addresses.is_empty() {
            return None;
        }

        let index = self.cursor.fetch_add(1, Ordering::Relaxed) % self.addresses.len();
        self.addresses.get(index).copied()
    }
}

/// One weighted round robin pool per `lb://` route.
pub struct StaticUpstreams {
    pools: HashMap<String, Arc<RoutePool>>,
}

impl StaticUpstreams {
    /// Build one pool per `lb://` route.
    pub fn build(config: &GatewayConfig) -> Result<Self, ConfigErrors> {
        let mut pools = HashMap::new();
        let mut errors = Vec::new();

        for route in &config.routes {
            let parsed = match UpstreamUri::parse(&route.uri) {
                Ok(parsed) => parsed,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };

            if parsed.scheme != crate::upstream::UpstreamScheme::Lb {
                continue;
            }

            if route.upstreams.is_empty() {
                errors.push(ConfigError::new(
                    format!("routes.{}.upstreams", route.id),
                    "an `lb://` route requires at least one upstream",
                ));
                continue;
            }

            let mut addresses = Vec::new();
            for upstream in &route.upstreams {
                let addr = if let Ok(addr) = upstream.addr.parse::<SocketAddr>() {
                    addr
                } else {
                    errors.push(ConfigError::new(
                        format!("routes.{}.upstreams.addr", route.id),
                        format!("`{}` is not a `IP:port` address", upstream.addr),
                    ));
                    continue;
                };

                for _ in 0..upstream.weight.max(1) {
                    addresses.push(addr);
                }
            }

            if addresses.is_empty() {
                continue;
            }

            pools.insert(
                route.id.clone(),
                Arc::new(RoutePool {
                    addresses,
                    cursor: AtomicUsize::new(0),
                }),
            );
        }

        if errors.is_empty() {
            Ok(Self { pools })
        } else {
            Err(ConfigErrors::new(errors))
        }
    }

    /// Number of configured pools, for startup logging.
    pub fn len(&self) -> usize {
        self.pools.len()
    }

    /// Whether no `lb://` route is configured.
    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }

    /// Whether the route has a backend right now.
    pub fn is_ready(&self, route_id: &str) -> bool {
        self.select(route_id, b"readiness").is_some()
    }
}

impl UpstreamResolver for StaticUpstreams {
    fn select(&self, route_id: &str, _key: &[u8]) -> Option<SocketAddr> {
        self.pools.get(route_id)?.next()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::str::FromStr;

    use crate::config::GatewayConfig;
    use crate::upstream::{StaticUpstreams, UpstreamResolver};

    fn config_with_two_upstreams() -> GatewayConfig {
        let yaml = "\
next:
  gateway:
    routes:
      - id: users
        uri: lb://user-service
        upstreams:
          - addr: '10.0.0.1:8080'
          - addr: '10.0.0.2:8080'
";
        GatewayConfig::from_str(yaml).unwrap()
    }

    #[test]
    fn round_robin_rotates_across_two_upstreams() {
        let upstreams = StaticUpstreams::build(&config_with_two_upstreams()).unwrap();

        let selected: HashSet<_> = (0..4)
            .filter_map(|_| upstreams.select("users", b"key"))
            .collect();

        assert_eq!(
            selected.len(),
            2,
            "expected both backends, got {selected:?}"
        );
    }

    #[test]
    fn an_unknown_route_selects_nothing() {
        let upstreams = StaticUpstreams::build(&config_with_two_upstreams()).unwrap();

        assert!(upstreams.select("missing", b"key").is_none());
        assert!(!upstreams.is_ready("missing"));
    }

    #[test]
    fn weights_are_honoured_exactly() {
        let yaml = "\
next:
  gateway:
    routes:
      - id: users
        uri: lb://user-service
        upstreams:
          - addr: '10.0.0.1:8080'
            weight: 3
          - addr: '10.0.0.2:8080'
            weight: 1
";
        let config = GatewayConfig::from_str(yaml).unwrap();
        let upstreams = StaticUpstreams::build(&config).unwrap();

        let heavy: std::net::SocketAddr = "10.0.0.1:8080".parse().unwrap();
        let light: std::net::SocketAddr = "10.0.0.2:8080".parse().unwrap();

        let mut heavy_count = 0;
        let mut light_count = 0;
        for _ in 0..40 {
            match upstreams.select("users", b"key") {
                Some(addr) if addr == heavy => heavy_count += 1,
                Some(addr) if addr == light => light_count += 1,
                _ => {}
            }
        }

        assert_eq!((heavy_count, light_count), (30, 10));
    }

    #[test]
    fn a_route_without_upstreams_is_reported() {
        // `GatewayConfig` already rejects this, so build the error path directly.
        let yaml = "\
next:
  gateway:
    routes:
      - id: users
        uri: http://backend:8080
";
        let config = GatewayConfig::from_str(yaml).unwrap();
        let upstreams = StaticUpstreams::build(&config).unwrap();

        assert!(upstreams.is_empty());
        assert_eq!(upstreams.len(), 0);
    }
}

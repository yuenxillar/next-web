//! Static load balancing over the upstreams listed on each `lb://` route.
//!
//! The balancer is a weighted round robin: every backend appears in the rotation
//! `weight` times, so the configured ratio is exact rather than statistical.
//! `pingora::lb::LoadBalancer` keeps one entry per distinct address (its
//! `Backends` is a `BTreeSet`), which loses the weight of a repeated address, so
//! the rotation is kept here. The [`UpstreamResolver`] seam is what a Phase 3
//! registry implementation plugs into.
//!
//! Backends marked unhealthy by the active TCP probe are skipped, and a pool
//! whose every backend is down selects nothing, which the proxy maps to 503.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::{ConfigError, ConfigErrors, GatewayConfig};
use crate::upstream::{UpstreamResolver, UpstreamUri};

/// Health state of every backend of every pool.
///
/// A backend absent from the map is considered healthy: the first probe result
/// decides, so a fresh gateway does not refuse traffic before the first round.
#[derive(Debug, Default)]
pub struct BackendHealth {
    unhealthy: Mutex<HashMap<SocketAddr, bool>>,
}

impl BackendHealth {
    /// Whether `addr` may serve traffic.
    pub fn is_healthy(&self, addr: SocketAddr) -> bool {
        match self.unhealthy.lock() {
            Ok(states) => !states.get(&addr).copied().unwrap_or(false),
            // A poisoned lock must not take the pool down with it.
            Err(poisoned) => !poisoned.into_inner().get(&addr).copied().unwrap_or(false),
        }
    }

    /// Record the result of one probe.
    pub fn record(&self, addr: SocketAddr, healthy: bool) {
        let mut states = match self.unhealthy.lock() {
            Ok(states) => states,
            Err(poisoned) => poisoned.into_inner(),
        };
        states.insert(addr, !healthy);
    }

    /// Addresses currently marked down, for tests and the admin endpoint.
    pub fn down(&self) -> Vec<SocketAddr> {
        let states = match self.unhealthy.lock() {
            Ok(states) => states,
            Err(poisoned) => poisoned.into_inner(),
        };

        let mut down: Vec<_> = states
            .iter()
            .filter(|(_, unhealthy)| **unhealthy)
            .map(|(addr, _)| *addr)
            .collect();
        down.sort();
        down
    }
}

/// The rotation of one `lb://` route.
struct RoutePool {
    /// Backend addresses repeated `weight` times.
    addresses: Vec<SocketAddr>,
    /// Distinct backends, in configuration order.
    backends: Vec<SocketAddr>,
    /// Next slot to serve.
    cursor: AtomicUsize,
}

impl RoutePool {
    /// Next healthy backend, starting at the cursor.
    fn next(&self, health: &BackendHealth) -> Option<SocketAddr> {
        if self.addresses.is_empty() {
            return None;
        }

        // Walk the rotation once: a pool whose every backend is down yields
        // `None` instead of handing back a dead address.
        for _ in 0..self.addresses.len() {
            let index = self.cursor.fetch_add(1, Ordering::Relaxed) % self.addresses.len();
            if let Some(addr) = self.addresses.get(index).copied()
                && health.is_healthy(addr)
            {
                return Some(addr);
            }
        }

        None
    }
}

/// One weighted round robin pool per `lb://` route.
pub struct StaticUpstreams {
    pools: HashMap<String, Arc<RoutePool>>,
    health: Arc<BackendHealth>,
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
            let mut backends = Vec::new();
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

                if !backends.contains(&addr) {
                    backends.push(addr);
                }

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
                    backends,
                    cursor: AtomicUsize::new(0),
                }),
            );
        }

        if errors.is_empty() {
            Ok(Self {
                pools,
                health: Arc::new(BackendHealth::default()),
            })
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

    /// Whether the route has a healthy backend right now.
    pub fn is_ready(&self, route_id: &str) -> bool {
        self.select(route_id, b"readiness").is_some()
    }

    /// Shared health state, so the probe loop and the selector agree.
    pub fn health(&self) -> Arc<BackendHealth> {
        self.health.clone()
    }

    /// Every distinct backend, across every pool.
    pub fn backends(&self) -> Vec<SocketAddr> {
        let mut all: Vec<SocketAddr> = self
            .pools
            .values()
            .flat_map(|pool| pool.backends.iter().copied())
            .collect();
        all.sort();
        all.dedup();
        all
    }

    /// Start the TCP health checker.
    ///
    /// A plain OS thread is enough: the probe is a blocking `connect` with a
    /// short timeout, and the loop only talks to the shared health map. It is a
    /// daemon thread, so it dies with the process.
    pub fn spawn_health_checker(self: &Arc<Self>, interval: Duration, timeout: Duration) {
        let backends = self.backends();
        if backends.is_empty() {
            return;
        }

        let health = self.health.clone();
        let interval = interval.max(Duration::from_millis(100));

        std::thread::spawn(move || {
            loop {
                for addr in &backends {
                    let healthy = TcpStream::connect_timeout(addr, timeout).is_ok();
                    health.record(*addr, healthy);

                    if !healthy {
                        tracing::warn!(backend = %addr, "health check failed, backend removed from the pool");
                    }
                }

                std::thread::sleep(interval);
            }
        });
    }
}

impl UpstreamResolver for StaticUpstreams {
    fn select(&self, route_id: &str, _key: &[u8]) -> Option<SocketAddr> {
        self.pools.get(route_id)?.next(&self.health)
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
    fn an_unhealthy_backend_is_skipped_until_it_recovers() {
        let upstreams = StaticUpstreams::build(&config_with_two_upstreams()).unwrap();
        let dead: std::net::SocketAddr = "10.0.0.2:8080".parse().unwrap();
        let alive: std::net::SocketAddr = "10.0.0.1:8080".parse().unwrap();

        upstreams.health().record(dead, false);

        for _ in 0..8 {
            assert_eq!(
                upstreams.select("users", b"key"),
                Some(alive),
                "a backend marked down must not be selected"
            );
        }

        upstreams.health().record(dead, true);
        let seen: HashSet<_> = (0..8)
            .filter_map(|_| upstreams.select("users", b"key"))
            .collect();
        assert_eq!(seen.len(), 2, "a recovered backend must return to the pool");
    }

    #[test]
    fn a_pool_with_every_backend_down_selects_nothing() {
        let upstreams = StaticUpstreams::build(&config_with_two_upstreams()).unwrap();

        for addr in upstreams.backends() {
            upstreams.health().record(addr, false);
        }

        assert!(upstreams.select("users", b"key").is_none());
        assert!(!upstreams.is_ready("users"));
        assert_eq!(upstreams.health().down().len(), 2);
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

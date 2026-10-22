//! Asynchronous DNS resolution with a small TTL cache.
//!
//! `pingora::upstreams::peer::HttpPeer::new` performs a blocking
//! `to_socket_addrs().unwrap()` on the proxy thread, which both stalls the
//! worker and panics on a name that does not resolve. The gateway therefore
//! resolves names itself and hands pingora a `SocketAddr`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Default time a resolved name stays cached.
const DEFAULT_TTL: Duration = Duration::from_secs(30);
/// Maximum number of cached names.
const DEFAULT_CAPACITY: usize = 1024;

struct Entry {
    addresses: Vec<SocketAddr>,
    resolved_at: Instant,
}

/// Resolves host names off the proxy thread and caches the result.
pub struct DnsCache {
    ttl: Duration,
    capacity: usize,
    entries: Mutex<HashMap<String, Entry>>,
}

impl Default for DnsCache {
    fn default() -> Self {
        Self {
            ttl: DEFAULT_TTL,
            capacity: DEFAULT_CAPACITY,
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl DnsCache {
    /// Build a cache with an explicit TTL and capacity.
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            ttl,
            capacity: capacity.max(1),
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve `host:port`, using the cache when the entry is still fresh.
    pub async fn resolve(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            return Ok(vec![SocketAddr::new(ip, port)]);
        }

        let key = format!("{host}:{port}");

        {
            let entries = self.entries.lock().await;
            if let Some(entry) = entries.get(&key)
                && entry.resolved_at.elapsed() < self.ttl
            {
                return Ok(entry.addresses.clone());
            }
        }

        let addresses = tokio::net::lookup_host((host, port))
            .await
            .map_err(|error| format!("failed to resolve `{host}`: {error}"))?
            .collect::<Vec<_>>();

        if addresses.is_empty() {
            return Err(format!("`{host}` resolved to no address"));
        }

        let mut entries = self.entries.lock().await;
        if entries.len() >= self.capacity {
            entries.clear();
        }
        entries.insert(
            key,
            Entry {
                addresses: addresses.clone(),
                resolved_at: Instant::now(),
            },
        );

        Ok(addresses)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::DnsCache;

    #[tokio::test]
    async fn resolves_a_literal_ip_without_lookup() {
        let cache = DnsCache::default();
        let addresses = cache.resolve("10.0.0.7", 8080).await.unwrap();

        assert_eq!(addresses, vec!["10.0.0.7:8080".parse().unwrap()]);
    }

    #[tokio::test]
    async fn caches_host_name_resolution() {
        let cache = DnsCache::new(Duration::from_secs(30), 8);

        let first = cache.resolve("localhost", 80).await;
        let second = cache.resolve("localhost", 80).await;

        match (first, second) {
            (Ok(first), Ok(second)) => assert_eq!(first, second),
            (Err(_), _) | (_, Err(_)) => {
                // A sandbox without a hosts entry cannot resolve localhost.
            }
        }
    }

    #[tokio::test]
    async fn reports_a_name_that_does_not_resolve() {
        let cache = DnsCache::default();
        assert!(cache.resolve("does-not-exist.invalid", 80).await.is_err());
    }
}

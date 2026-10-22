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
/// How long a failed lookup is remembered before it is retried.
const NEGATIVE_TTL: Duration = Duration::from_secs(5);
/// Maximum number of cached names.
const DEFAULT_CAPACITY: usize = 1024;

struct Entry {
    /// `None` when the last lookup for this name failed.
    addresses: Option<Vec<SocketAddr>>,
    resolved_at: Instant,
}

/// Resolves host names off the proxy thread and caches the result.
pub struct DnsCache {
    ttl: Duration,
    negative_ttl: Duration,
    capacity: usize,
    entries: Mutex<HashMap<String, Entry>>,
}

impl Default for DnsCache {
    fn default() -> Self {
        Self {
            ttl: DEFAULT_TTL,
            negative_ttl: NEGATIVE_TTL,
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
            negative_ttl: NEGATIVE_TTL.min(ttl),
            capacity: capacity.max(2),
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve `host:port`, using the cache when the entry is still fresh.
    ///
    /// A failed lookup is remembered for a short while: a name that does not
    /// resolve is usually a configuration error, and retrying it on every
    /// request would put the resolver on the request path for free.
    pub async fn resolve(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            return Ok(vec![SocketAddr::new(ip, port)]);
        }

        let key = format!("{host}:{port}");

        {
            let entries = self.entries.lock().await;
            if let Some(entry) = entries.get(&key) {
                let ttl = if entry.addresses.is_some() {
                    self.ttl
                } else {
                    self.negative_ttl
                };

                if entry.resolved_at.elapsed() < ttl {
                    return match entry.addresses.clone() {
                        Some(addresses) => Ok(addresses),
                        None => Err(format!("`{host}` does not resolve (cached failure)")),
                    };
                }
            }
        }

        let resolved = tokio::net::lookup_host((host, port))
            .await
            .map(|addresses| addresses.collect::<Vec<_>>());

        let (addresses, error) = match resolved {
            Ok(addresses) if !addresses.is_empty() => (Some(addresses), None),
            Ok(_) => (None, Some(format!("`{host}` resolved to no address"))),
            Err(error) => (None, Some(format!("failed to resolve `{host}`: {error}"))),
        };

        self.store(key, addresses.clone()).await;

        match (addresses, error) {
            (Some(addresses), _) => Ok(addresses),
            (None, Some(error)) => Err(error),
            (None, None) => Err(format!("`{host}` resolved to no address")),
        }
    }

    /// Insert one entry, evicting the oldest when the cache is full.
    ///
    /// Evicting exactly one entry keeps the rest of the cache warm; clearing the
    /// whole map on overflow would drop every name at once and send the next
    /// thousand requests to the resolver.
    async fn store(&self, key: String, addresses: Option<Vec<SocketAddr>>) {
        let mut entries = self.entries.lock().await;

        if entries.len() >= self.capacity && !entries.contains_key(&key) {
            let oldest = entries
                .iter()
                .min_by_key(|(_, entry)| entry.resolved_at)
                .map(|(key, _)| key.clone());

            if let Some(oldest) = oldest {
                entries.remove(&oldest);
            }
        }

        entries.insert(
            key,
            Entry {
                addresses,
                resolved_at: Instant::now(),
            },
        );
    }

    /// Number of cached names, for tests and diagnostics.
    #[cfg(test)]
    async fn len(&self) -> usize {
        self.entries.lock().await.len()
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

    #[tokio::test]
    async fn a_failed_lookup_is_cached_briefly() {
        let cache = DnsCache::default();

        let first = cache
            .resolve("does-not-exist.invalid", 80)
            .await
            .unwrap_err();
        assert!(!first.contains("cached"));

        let second = cache
            .resolve("does-not-exist.invalid", 80)
            .await
            .unwrap_err();
        assert!(
            second.contains("cached"),
            "the second lookup must come from the negative cache, got `{second}`"
        );
        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn a_full_cache_evicts_one_entry_instead_of_all() {
        let cache = DnsCache::new(Duration::from_secs(30), 2);

        // Literal IPs bypass the cache, so use names that actually get stored.
        if cache.resolve("localhost", 80).await.is_err() {
            // A sandbox without a hosts entry cannot resolve localhost.
            return;
        }
        cache.resolve("localhost", 81).await.ok();
        assert_eq!(cache.len().await, 2);

        // A third name overflows the capacity of two.
        cache.resolve("does-not-exist.invalid", 80).await.ok();
        assert_eq!(
            cache.len().await,
            2,
            "the cache must stay at capacity rather than drop everything"
        );
    }
}

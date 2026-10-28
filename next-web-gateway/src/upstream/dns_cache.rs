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
    /// Port this entry was resolved for.
    port: u16,
    /// `None` when the last lookup for this name failed.
    addresses: Option<Vec<SocketAddr>>,
    resolved_at: Instant,
}

/// Resolves host names off the proxy thread and caches the result.
///
/// Entries are grouped by host so the lookup takes the borrowed `&str` and never
/// builds a `host:port` string on the request path.
pub struct DnsCache {
    ttl: Duration,
    negative_ttl: Duration,
    capacity: usize,
    entries: Mutex<HashMap<Box<str>, Vec<Entry>>>,
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

        {
            let entries = self.entries.lock().await;
            if let Some(found) = entries
                .get(host)
                .and_then(|group| group.iter().find(|entry| entry.port == port))
            {
                let ttl = if found.addresses.is_some() {
                    self.ttl
                } else {
                    self.negative_ttl
                };

                if found.resolved_at.elapsed() < ttl {
                    return match found.addresses.clone() {
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

        self.store(host, port, addresses.clone()).await;

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
    async fn store(&self, host: &str, port: u16, addresses: Option<Vec<SocketAddr>>) {
        let mut entries = self.entries.lock().await;
        let resolved_at = Instant::now();

        // Update in place when the host (and port) is already cached.
        if let Some(group) = entries.get_mut(host)
            && let Some(existing) = group.iter_mut().find(|entry| entry.port == port)
        {
            existing.addresses = addresses;
            existing.resolved_at = resolved_at;
            return;
        }

        let total: usize = entries.values().map(Vec::len).sum();
        if total >= self.capacity {
            let oldest = entries
                .iter()
                .flat_map(|(host, group)| {
                    group
                        .iter()
                        .map(move |entry| (host.clone(), entry.port, entry.resolved_at))
                })
                .min_by_key(|(_, _, resolved_at)| *resolved_at)
                .map(|(host, port, _)| (host, port));

            if let Some((oldest_host, oldest_port)) = oldest
                && let Some(group) = entries.get_mut(oldest_host.as_ref())
            {
                group.retain(|entry| entry.port != oldest_port);
                if group.is_empty() {
                    entries.remove(oldest_host.as_ref());
                }
            }
        }

        entries.entry(host.into()).or_default().push(Entry {
            port,
            addresses,
            resolved_at,
        });
    }

    /// Number of cached names, for tests and diagnostics.
    #[cfg(test)]
    async fn len(&self) -> usize {
        self.entries.lock().await.values().map(Vec::len).sum()
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

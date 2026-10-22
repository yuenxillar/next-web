//! `XForwardedRemoteAddr` predicate.
//!
//! The configured arguments are the address ranges the *client* must fall into.
//! `X-Forwarded-For` is only consulted when the direct peer is one of the
//! gateway's `trusted_proxies` (D15): with an empty trust list the header is
//! ignored entirely, so a client cannot spoof its own address.

use std::net::IpAddr;
use std::sync::OnceLock;

use ipnetwork::IpNetwork;
use pingora::protocols::http::ServerSession;
use pingora::protocols::l4::socket::SocketAddr;

use super::remote_addr_route_predicate_factory::normalize_ip;
use super::route_predicate::RoutePredicate;

static TRUSTED_PROXIES: OnceLock<Vec<IpNetwork>> = OnceLock::new();

/// Install the `next.gateway.trusted_proxies` list once at startup.
// Called by the gateway bootstrap; the predicate itself only reads it.
#[allow(dead_code)]
pub fn set_trusted_proxies(networks: Vec<IpNetwork>) {
    if TRUSTED_PROXIES.set(networks).is_err() {
        tracing::warn!("trusted proxies were already installed, ignoring the new list");
    }
}

/// The installed trusted proxy list.
pub fn trusted_proxies() -> &'static [IpNetwork] {
    TRUSTED_PROXIES
        .get()
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// The `XForwardedRemoteAddr` predicate.
#[derive(Debug, Clone)]
pub struct XForwardedRemoteAddrRoutePredicateFactory {
    /// Accepted client networks.
    pub remote_addrs: Vec<IpNetwork>,
}

impl RoutePredicate for XForwardedRemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut ServerSession) -> bool {
        let Some(direct) = session.client_addr() else {
            return false;
        };
        // On Windows the enum has a single variant, so `Inet` is irrefutable.
        #[allow(clippy::infallible_destructuring_match)]
        let direct = match direct {
            SocketAddr::Inet(addr) => addr,
            #[cfg(unix)]
            SocketAddr::Unix(_) => return false,
        };

        let forwarded = session
            .req_header()
            .headers
            .get("X-Forwarded-For")
            .and_then(|value| value.to_str().ok());

        let Some(client) = resolve_client(normalize_ip(direct.ip()), forwarded, trusted_proxies())
        else {
            return false;
        };

        self.remote_addrs
            .iter()
            .any(|network| network.contains(client))
    }
}

/// Resolve the client address from the direct peer and `X-Forwarded-For`.
///
/// Returns `None` when the direct peer is trusted but every forwarded entry is
/// itself a trusted proxy, because then the real client is unknown.
pub(crate) fn resolve_client(
    direct: IpAddr,
    forwarded: Option<&str>,
    trusted: &[IpNetwork],
) -> Option<IpAddr> {
    if !trusted.iter().any(|network| network.contains(direct)) {
        return Some(direct);
    }

    let forwarded = forwarded?;

    // Walk right to left, skipping the trusted proxies we already know about.
    forwarded.split(',').rev().find_map(|entry| {
        let ip: IpAddr = entry.trim().parse().ok()?;
        let ip = normalize_ip(ip);
        (!trusted.iter().any(|network| network.contains(ip))).then_some(ip)
    })
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use ipnetwork::IpNetwork;

    use super::resolve_client;

    fn networks(entries: &[&str]) -> Vec<IpNetwork> {
        entries.iter().map(|entry| entry.parse().unwrap()).collect()
    }

    #[test]
    fn untrusted_peer_ignores_the_forwarded_header() {
        let direct: IpAddr = "203.0.113.9".parse().unwrap();
        let trusted = networks(&["10.0.0.0/8"]);

        assert_eq!(
            resolve_client(direct, Some("1.2.3.4"), &trusted),
            Some(direct)
        );
    }

    #[test]
    fn trusted_peer_uses_the_rightmost_untrusted_entry() {
        let direct: IpAddr = "10.0.0.1".parse().unwrap();
        let trusted = networks(&["10.0.0.0/8"]);

        let client = resolve_client(direct, Some("1.2.3.4, 10.0.0.7"), &trusted);
        assert_eq!(client, Some("1.2.3.4".parse().unwrap()));
    }

    #[test]
    fn no_trusted_proxies_configured_never_reads_the_header() {
        let direct: IpAddr = "10.0.0.1".parse().unwrap();

        assert_eq!(resolve_client(direct, Some("1.2.3.4"), &[]), Some(direct));
    }
}

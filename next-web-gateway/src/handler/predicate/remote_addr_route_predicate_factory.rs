//! `RemoteAddr` predicate: matches the TCP peer address.

use std::net::IpAddr;

use ipnetwork::IpNetwork;
use pingora::protocols::l4::socket::SocketAddr;

use super::route_predicate::RoutePredicate;

/// The `RemoteAddr` predicate.
#[derive(Debug, Clone)]
pub struct RemoteAddrRoutePredicateFactory {
    /// Accepted peer networks.
    pub remote_addrs: Vec<IpNetwork>,
}

impl RoutePredicate for RemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let Some(client_addr) = session.client_addr() else {
            return false;
        };

        let ip = match client_addr {
            SocketAddr::Inet(socket_addr) => normalize_ip(socket_addr.ip()),
            #[cfg(unix)]
            SocketAddr::Unix(_) => return false,
        };

        self.remote_addrs.iter().any(|network| network.contains(ip))
    }
}

/// Collapse an IPv4-mapped IPv6 address onto its IPv4 form.
///
/// A dual stack listener reports `::ffff:10.0.0.5`, which would never match a
/// `10.0.0.0/8` network without this normalisation.
pub(crate) fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        IpAddr::V4(v4) => IpAddr::V4(v4),
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use ipnetwork::IpNetwork;

    use super::normalize_ip;

    #[test]
    fn ipv4_mapped_ipv6_peer_matches_ipv4_cidr() {
        let peer: IpAddr = "::ffff:10.0.0.5".parse().unwrap();
        let network: IpNetwork = "10.0.0.0/8".parse().unwrap();

        assert!(network.contains(normalize_ip(peer)));
    }

    #[test]
    fn plain_ipv4_is_unchanged() {
        let peer: IpAddr = "192.168.1.1".parse().unwrap();
        assert_eq!(normalize_ip(peer), peer);
    }
}

use ipnetwork::IpNetwork;
use pingora::protocols::l4::socket::SocketAddr;

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct RemoteAddrRoutePredicateFactory {
    pub remote_addrs: Vec<IpNetwork>,
}

impl RoutePredicate for RemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let client_addr = match session.client_addr() {
            Some(addr) => addr,
            None => return false,
        };

        let ip = match client_addr {
            SocketAddr::Inet(socket_addr) => socket_addr.ip(),
            #[cfg(unix)]
            SocketAddr::Unix(_) => return false,
        };

        self.remote_addrs.iter().any(|network| network.contains(ip))
    }
}

//! `XForwardedRemoteAddr` predicate.
//!
//! The configured arguments are the address ranges the *client* must fall into.
//! `X-Forwarded-For` is only consulted when the direct peer is one of the
//! gateway's `trusted_proxies` (D15), so with an empty list the header is
//! ignored entirely and a client cannot spoof its own address.

use ipnetwork::IpNetwork;
use pingora::protocols::http::ServerSession;
use pingora::protocols::l4::socket::SocketAddr;

use super::route_predicate::RoutePredicate;
use crate::util::forwarded::{resolve_client, trusted_proxies};

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

        let Some(client) = resolve_client(direct.ip(), forwarded, trusted_proxies()) else {
            return false;
        };

        self.remote_addrs
            .iter()
            .any(|network| network.contains(client))
    }
}

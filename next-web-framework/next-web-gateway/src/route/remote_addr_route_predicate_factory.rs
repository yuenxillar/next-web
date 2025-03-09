use crate::utils::str_util::StrUtil;

use super::route_predicate::RoutePredicate;
use pingora::protocols::l4::socket::SocketAddr::Inet;

#[derive(Debug, Clone)]
pub struct RemoteAddrRoutePredicateFactory {
    pub remote_addr: String,
}

impl RoutePredicate for RemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        if let Some(client_addr) = session.client_addr() {
            if let Inet(addr) = client_addr {
                if StrUtil::host_match(&self.remote_addr, &addr.ip().to_string()) {
                    return true;
                }
            }
        }
        false
    }
}

use crate::utils::str_util::StrUtil;

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct HostRoutePredicateFactory {
    pub hosts: Vec<String>,
}

impl RoutePredicate for HostRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        if let Some(remote_addr) = session.req_header().headers.get("Host") {
            if let Ok(remote_addr) = remote_addr.to_str() {
                if remote_addr.is_empty() {
                    return false;
                }

                for host in self.hosts.iter() {
                    if host.contains("*") {
                        if StrUtil::host_match(remote_addr, &host) {
                            return true;
                        }
                    } else {
                        if host.eq(remote_addr) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

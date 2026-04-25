use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use ipnetwork::IpNetwork;
use pingora::protocols::http::ServerSession;

use crate::route::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct XForwardedRemoteAddrRoutePredicateFactory {
    pub trusted_networks: Vec<IpNetwork>,
}

impl RoutePredicate for XForwardedRemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut ServerSession) -> bool {
        // Extract client IP
        let client_ip = match self.extract_client_ip_from_x_forwarded_for(session) {
            Some(ip) => ip,
            None => return false,
        };

        // Check if the IP is in any trusted network
        for network in &self.trusted_networks {
            if network.contains(client_ip) {
                return true;
            }
        }

        false
    }
}

impl XForwardedRemoteAddrRoutePredicateFactory {
    fn extract_client_ip_from_x_forwarded_for(&self, session: &ServerSession) -> Option<IpAddr> {
        // Get X-Forwarded-For header
        let req_header = session.req_header();
        let x_forwarded_for = req_header.headers.get("X-Forwarded-For")?;

        // X-Forwarded-For may contain multiple IPs (separated by commas), take the first one (the leftmost client IP)
        let first_ip_str = x_forwarded_for.to_str().ok()?.split(',').next()?.trim();

        // Parse IP address
        if let Ok(ipv4) = Ipv4Addr::from_str(first_ip_str) {
            Some(IpAddr::V4(ipv4))
        } else if let Ok(ipv6) = Ipv6Addr::from_str(first_ip_str) {
            Some(IpAddr::V6(ipv6))
        } else {
            None
        }
    }
}

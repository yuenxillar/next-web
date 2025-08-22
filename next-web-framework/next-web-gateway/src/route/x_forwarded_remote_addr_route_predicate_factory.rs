use std::{net::{IpAddr, Ipv4Addr, Ipv6Addr}, str::FromStr};

use ipnetwork::IpNetwork;
use pingora::protocols::http::ServerSession;

use crate::route::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct XForwardedRemoteAddrRoutePredicateFactory {
    pub trusted_networks: Vec<IpNetwork>,
}

impl RoutePredicate for XForwardedRemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut ServerSession) -> bool {

        // 提取客户端IP
        let client_ip = match self.extract_client_ip_from_x_forwarded_for(session) {
            Some(ip) => ip,
            None => return false
        };

        // 检查IP是否在任何信任的网络中
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
        // 获取 X-Forwarded-For header
        let req_header = session.req_header();
        let x_forwarded_for = req_header.headers.get("X-Forwarded-For")?;

        // X-Forwarded-For 可能包含多个IP（逗号分隔），取第一个（最左边的客户端IP）
        let first_ip_str = x_forwarded_for.to_str().ok()?.split(',').next()?.trim();

        // 解析IP地址
        if let Ok(ipv4) = Ipv4Addr::from_str(first_ip_str) {
            Some(IpAddr::V4(ipv4))
        } else if let Ok(ipv6) = Ipv6Addr::from_str(first_ip_str) {
            Some(IpAddr::V6(ipv6))
        } else {
            None
        }
    }
}
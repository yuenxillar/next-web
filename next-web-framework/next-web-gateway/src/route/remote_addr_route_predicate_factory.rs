use crate::util::str_util::StrUtil;

use super::route_predicate::RoutePredicate;
use pingora::protocols::l4::socket::SocketAddr::Inet;

#[derive(Debug, Clone)]
pub struct RemoteAddrRoutePredicateFactory {
    pub remote_addr: String,
}

impl RoutePredicate for RemoteAddrRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        // 1. 获取客户端地址
        let client_addr = match session.client_addr() {
            Some(addr) => addr,
            None => return false,
        };

        // 2. 提取 IP 地址
        let ip = match client_addr {
            std::net::SocketAddr::V4(v4) => v4.ip().into(),
            std::net::SocketAddr::V6(v6) => v6.ip().into(),
        };

        // 3. 转换为字符串并进行匹配
        StrUtil::host_match(&self.remote_addr, &ip.to_string())
    }
}

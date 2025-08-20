use crate::util::str_util::StrUtil;
use super::route_predicate::RoutePredicate;
use pingora::protocols::http::ServerSession;

#[derive(Debug, Clone)]
pub struct HostRoutePredicateFactory {
    pub hosts: Vec<String>,
}

impl RoutePredicate for HostRoutePredicateFactory {
    fn matches(&self, session: &mut ServerSession) -> bool {
        // 1. 使用更简洁的方式获取Host头
        let Some(host_header) = session.req_header().headers.get("Host") else {
            return false;
        };

        // 2. 使用if let Ok优化错误处理
        let Ok(remote_host) = host_header.to_str() else {
            return false;
        };

        // 3. 提前检查空值
        if remote_host.trim().is_empty() {
            return false;
        }

        // 4. 使用迭代器方法提高性能和可读性
        self.hosts.iter().any(|host| {
            if host.contains('*') {
                StrUtil::host_match(remote_host, host)
            } else {
                host == remote_host
            }
        })
    }
}
use crate::util::str::StrUtil;
use pingora::protocols::http::ServerSession;

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct HostRoutePredicateFactory {
    pub hosts: Vec<String>,
}

impl RoutePredicate for HostRoutePredicateFactory {
    fn matches(&self, session: &mut ServerSession) -> bool {
        // HTTP/1.0 requests and some proxies omit `Host`; the request target's
        // authority is the fallback (D11).
        let request = session.req_header();
        let remote_host = match request.headers.get("Host") {
            Some(value) => value.to_str().map(str::to_string).ok(),
            None => request.uri.authority().map(|authority| authority.to_string()),
        };
        let Some(remote_host) = remote_host else {
            return false;
        };

        let remote_host = normalize_host(&remote_host);
        if remote_host.is_empty() {
            return false;
        }

        self.hosts.iter().any(|host| {
            let host = normalize_host(host);
            if host.contains('*') {
                StrUtil::host_match(&remote_host, &host)
            } else {
                host == remote_host
            }
        })
    }
}

fn normalize_host(host: &str) -> String {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return host;
    }

    if let Some(stripped) = host.strip_prefix('[')
        && let Some((ipv6, _)) = stripped.split_once(']')
    {
        return ipv6.to_string();
    }

    if host.matches(':').count() == 1 {
        return host
            .split_once(':')
            .map(|(name, _)| name.to_string())
            .unwrap_or(host);
    }

    host
}

#[cfg(test)]
mod tests {
    use super::normalize_host;

    #[test]
    fn normalize_host_removes_port_and_case() {
        assert_eq!(normalize_host("Example.COM:8080"), "example.com");
    }

    #[test]
    fn normalize_host_handles_bracketed_ipv6() {
        assert_eq!(normalize_host("[2001:db8::1]:8080"), "2001:db8::1");
    }
}

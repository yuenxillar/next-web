//! `RequestRateLimiter` filter: fixed window admission control per client.

use std::sync::Arc;

use async_trait::async_trait;
use pingora_limits::rate::Rate;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;

/// Limits how many requests one client may issue per second.
#[derive(Clone)]
pub struct RequestRateLimiterFilter {
    /// Requests allowed per window.
    pub rate_limit: u32,
    /// Sliding estimator; the current window count is authoritative.
    pub limiter: Arc<Rate>,
}

impl std::fmt::Debug for RequestRateLimiterFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestRateLimiterFilter")
            .field("rate_limit", &self.rate_limit)
            .finish()
    }
}

#[async_trait]
impl GatewayFilter for RequestRateLimiterFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let context = exchange.request_context();
        let key = rate_limit_key(context.route_id.as_deref(), context.client_ip);

        // `Rate::rate()` reports the previous window, so it lags by a second and
        // lets a burst through. `observe()` returns the count seen so far in the
        // current window, which is what a fixed window limiter needs.
        let seen = self.limiter.observe(&key, 1);

        if seen > self.rate_limit as isize {
            return exchange
                .request_context()
                .respond_with_text(
                    429,
                    vec![
                        ("Retry-After".to_string(), "1".to_string()),
                        (
                            "X-RateLimit-Limit".to_string(),
                            self.rate_limit.to_string(),
                        ),
                        ("X-RateLimit-Remaining".to_string(), "0".to_string()),
                    ],
                    "Too many requests",
                )
                .map_err(|_| GatewayError::ServerRejectsRequest);
        }

        chain.filter(exchange).await
    }
}

/// One bucket per client address, never one bucket per route.
pub(crate) fn rate_limit_key(route_id: Option<&str>, client_ip: Option<std::net::IpAddr>) -> String {
    match client_ip {
        Some(ip) => ip.to_string(),
        None => format!("unresolved:{}", route_id.unwrap_or("global")),
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::rate_limit_key;

    #[test]
    fn rate_limiter_buckets_per_client_address() {
        let first: IpAddr = "203.0.113.1".parse().unwrap();
        let second: IpAddr = "203.0.113.2".parse().unwrap();

        assert_ne!(
            rate_limit_key(Some("route"), Some(first)),
            rate_limit_key(Some("route"), Some(second))
        );
        assert_eq!(rate_limit_key(Some("route"), Some(first)), "203.0.113.1");
    }

    #[test]
    fn rate_limiter_falls_back_to_a_route_bucket_without_a_client() {
        assert_eq!(rate_limit_key(Some("users"), None), "unresolved:users");
        assert_eq!(rate_limit_key(None, None), "unresolved:global");
    }
}

use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use std::fmt;
use std::sync::Arc;

use pingora_limits::rate::Rate;

#[derive(Clone)]
pub struct RequestRateLimiterFilter {
    pub rate_limit: u32,
    pub limiter: Arc<Rate>,
}

impl fmt::Debug for RequestRateLimiterFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestRateLimiterFilter")
            .field("rate_limit", &self.rate_limit)
            .finish()
    }
}

#[async_trait]
#[async_trait]
impl GatewayFilter for RequestRateLimiterFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        if self.rate_limit == 0 {
            return chain.filter(exchange).await;
        }

        // Use the route id as the limiter bucket so each route can enforce its own QPS cap.
        let key = exchange.request_context().route_id.clone().unwrap_or_else(|| "global".to_string());

        self.limiter.observe(&key, 1);
        if self.limiter.rate(&key) > self.rate_limit as f64 {
            return exchange.request_context().respond_with_text(
                429,
                vec![
                    ("Retry-After".to_string(), "1".to_string()),
                    (
                        "X-Rate-Limit-Limit".to_string(),
                        self.rate_limit.to_string(),
                    ),
                ],
                "Too many requests",
            ).map_err(|_| GatewayError::ServerRejectsRequest);
        }

        chain.filter(exchange).await
    }
}

use std::fmt;
use std::sync::Arc;

use pingora_limits::rate::Rate;

use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

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

impl GatewayFilter for RequestRateLimiterFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        _upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        if self.rate_limit == 0 {
            return Ok(());
        }

        // Use the route id as the limiter bucket so each route can enforce its own QPS cap.
        let key = ctx.route_id.clone().unwrap_or_else(|| "global".to_string());

        self.limiter.observe(&key, 1);
        if self.limiter.rate(&key) > self.rate_limit as f64 {
            return ctx.respond_with_text(
                429,
                vec![
                    ("Retry-After".to_string(), "1".to_string()),
                    (
                        "X-Rate-Limit-Limit".to_string(),
                        self.rate_limit.to_string(),
                    ),
                ],
                "Too many requests",
            );
        }

        Ok(())
    }
}

//! `CircuitBreaker` filter: binds a route to a breaker defined under `breakers`.
//!
//! The filter itself does not change the exchange. `upstream_peer` reserves the
//! probe through the registry and `response_filter` records the outcome, so the
//! filter exists to declare the binding, to validate the id at startup and to
//! make the breaker visible in `/routes`.

use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;

/// Declares which breaker guards the route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitBreakerFilter {
    /// Id of the breaker under `next.gateway.breakers`.
    pub breaker_id: String,
}

#[async_trait]
impl GatewayFilter for CircuitBreakerFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        chain.filter(exchange).await
    }
}

use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct RedirectToFilter {
    pub status: u16,
    pub url: Box<str>,
}

#[async_trait]
#[async_trait]
impl GatewayFilter for RedirectToFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        _chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        // Short-circuit the proxy flow and send a redirect directly to the downstream client.
        exchange.request_context().respond_with_empty(
            self.status,
            vec![("Location".to_string(), self.url.to_string())],
        ).map_err(|_| GatewayError::ServerRejectsRequest)
    }
}

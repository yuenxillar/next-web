use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct SetRequestHostHeaderFilter {
    pub host: String,
}

#[async_trait]
impl GatewayFilter for SetRequestHostHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        exchange.request_header().map(|request_header| {
            // Set the upstream Host header explicitly so H1 and H2 forwarding use the same host.
            request_header
                .insert_header("Host".to_string(), self.host.as_str())
                .ok();
        });

        chain.filter(exchange).await
    }
}

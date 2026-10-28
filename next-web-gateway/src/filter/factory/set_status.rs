use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct SetStatusFilter {
    pub status: u16,
}

#[async_trait]
impl GatewayFilter for SetStatusFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        match exchange.response_header() {
            Some(response_header) => {
                response_header.set_status(self.status).ok();
            }
            None => return chain.filter(exchange).await,
        }

        chain.filter(exchange).await
    }
}

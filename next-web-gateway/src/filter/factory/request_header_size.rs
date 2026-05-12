use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct RequestHeaderSizeFilter {
    pub max_size: u32,
    pub error_message: String,
}

#[async_trait]
impl GatewayFilter for RequestHeaderSizeFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        if self.max_size == 0 {
            return chain.filter(exchange).await;
        }

        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        // Estimate the serialized header size so oversized requests can be rejected early.
        let header_size = request_header
            .headers
            .iter()
            .map(|(name, value)| name.as_str().len() + value.as_bytes().len() + 4)
            .sum::<usize>()
            + 2;

        if header_size > self.max_size as usize {
            return exchange
                .request_context()
                .respond_with_text(431, Vec::new(), self.error_message.clone())
                .map_err(|_| GatewayError::ServerRejectsRequest);
        }

        chain.filter(exchange).await
    }
}
